#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue,
	Home, HomeComponent, HomeComponentValue, HomeLayout, ImageRequestProvider, Listing,
	ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, MultiSelectFilter, Page,
	PageContent, PageContext, Result, Source, UpdateStrategy, Viewer,
	alloc::{String, Vec, borrow::Cow, format, vec},
	helpers::uri::{decode_uri, encode_uri_component},
	imports::{
		html::{Document, Element},
		net::{Request, TimeUnit, set_rate_limit},
		std::send_partial_result,
	},
	prelude::*,
};

const BASE_URL: &str = "https://www.freecomics.xxx";
type FacetOptions = Vec<(String, String)>;

struct FreeComicsXxx;

#[derive(Default)]
struct SearchFacets {
	included_genres: Vec<String>,
	excluded_genres: Vec<String>,
	included_artists: Vec<String>,
	excluded_artists: Vec<String>,
}

enum Identity<'a> {
	Series(&'a str),
	Book(&'a str),
}

impl FreeComicsXxx {
	fn request(&self, path: &str) -> Result<Request> {
		let url = if path.starts_with("http") {
			path.into()
		} else {
			format!("{BASE_URL}{path}")
		};
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/main1.html")))
	}

	fn identity(key: &str) -> Identity<'_> {
		if let Some(value) = key.strip_prefix("series--") {
			Identity::Series(value)
		} else {
			Identity::Book(key.strip_prefix("book--").unwrap_or(key))
		}
	}

	fn route(key: &str) -> String {
		match Self::identity(key) {
			Identity::Series(value) => format!("/series-{value}-page-1.html"),
			Identity::Book(value) => format!("/books/{value}.html"),
		}
	}

	fn destination(href: &str) -> String {
		let tracked = href
			.split('?')
			.nth(1)
			.and_then(|query| query.split('&').find_map(|pair| pair.strip_prefix("url=")));
		tracked.map(decode_uri).unwrap_or_else(|| href.into())
	}

	fn extract_between<'a>(value: &'a str, start: &str, end: &str) -> Option<&'a str> {
		let rest = value.split_once(start)?.1;
		Some(rest.split_once(end).map(|(v, _)| v).unwrap_or(rest))
	}

	fn clean(value: String) -> String {
		value.split_whitespace().collect::<Vec<_>>().join(" ")
	}

	fn route_slug(href: &str, kind: &str) -> Option<String> {
		let destination = Self::destination(href);
		Self::extract_between(&destination, &format!("/{kind}-"), "-page-")
			.map(|value| value.to_ascii_lowercase())
	}

	fn card_facets(card: &Element) -> (Vec<String>, Vec<String>) {
		let genres = card
			.select("a[href*='genre-']")
			.map(|links| {
				links
					.filter_map(|link| Self::route_slug(&link.attr("href")?, "genre"))
					.collect()
			})
			.unwrap_or_default();
		let artists = card
			.select("a[href*='artist-']")
			.map(|links| {
				links
					.filter_map(|link| Self::route_slug(&link.attr("href")?, "artist"))
					.collect()
			})
			.unwrap_or_default();
		(genres, artists)
	}

	fn card_matches(card: &Element, query: Option<&str>, facets: &SearchFacets) -> bool {
		let text = card.text().unwrap_or_default().to_ascii_lowercase();
		if query.is_some_and(|query| !text.contains(&query.trim().to_ascii_lowercase())) {
			return false;
		}
		let (genres, artists) = Self::card_facets(card);
		let includes = facets
			.included_genres
			.iter()
			.all(|id| genres.is_empty() || genres.contains(id))
			&& facets
				.included_artists
				.iter()
				.all(|id| artists.is_empty() || artists.contains(id));
		let excludes = facets.excluded_genres.iter().any(|id| genres.contains(id))
			|| facets
				.excluded_artists
				.iter()
				.any(|id| artists.contains(id));
		includes && !excludes
	}

	fn parse_cards(
		&self,
		html: &Document,
		query: Option<&str>,
		facets: &SearchFacets,
	) -> (Vec<Manga>, usize) {
		let raw_count = html
			.select(".xcpreview")
			.map(|cards| cards.count())
			.unwrap_or_default();
		html.select(".xcpreview")
			.map(|cards| {
				let mut entries: Vec<Manga> = Vec::new();
				for manga in cards.filter_map(|card| {
					if !Self::card_matches(&card, query, facets) {
						return None;
					}
					let main = card.select_first("a[href*='/books/']")?;
					let href = main.attr("href").or_else(|| main.attr("title"))?;
					let destination = Self::destination(&href);
					let book_id = Self::extract_between(&destination, "/books/", ".html")?;
					let series = card
						.select_first("a[href*='/series-']")
						.and_then(|e| e.attr("href"))
						.and_then(|href| {
							Self::extract_between(&href, "/series-", "-page-").map(String::from)
						});
					let key = series
						.map(|v| format!("series--{v}"))
						.unwrap_or_else(|| format!("book--{book_id}"));
					let image = main
						.select_first("img")
						.or_else(|| card.select_first("img"));
					let cover = image.as_ref().and_then(|e| {
						e.attr("data-src")
							.or_else(|| e.attr("data-lazy-src"))
							.or_else(|| e.attr("data-original"))
							.or_else(|| e.attr("abs:src"))
					});
					let raw_title = main
						.attr("title")
						.or_else(|| image.and_then(|e| e.attr("alt")))
						.or_else(|| card.select_first(".bookinfo").and_then(|e| e.text()))?;
					let mut title = Self::clean(raw_title);
					if let Some((before, _)) = title.split_once("(Chapter") {
						title = before.trim().into();
					}
					Some(Manga {
						key,
						title,
						cover,
						content_rating: ContentRating::NSFW,
						viewer: Viewer::Webtoon,
						..Default::default()
					})
				}) {
					if !entries.iter().any(|item| item.key == manga.key) {
						entries.push(manga);
					}
				}
				(entries, raw_count)
			})
			.unwrap_or_else(|| (Vec::new(), 0))
	}

	fn search(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut facets = SearchFacets::default();
		for filter in filters {
			match filter {
				FilterValue::Text { id, value } if id == "genre" && !value.trim().is_empty() => {
					facets
						.included_genres
						.push(value.trim().to_ascii_lowercase())
				}
				FilterValue::Text { id, value } if id == "artist" && !value.trim().is_empty() => {
					facets
						.included_artists
						.push(value.trim().to_ascii_lowercase())
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} if id == "genre" => {
					facets.included_genres = included;
					facets.excluded_genres = excluded;
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} if id == "artist" => {
					facets.included_artists = included;
					facets.excluded_artists = excluded;
				}
				_ => {}
			}
		}
		let path = if let Some(query) = query.as_deref().filter(|q| !q.trim().is_empty()) {
			format!("/?search={}", encode_uri_component(query.trim()))
		} else if let Some(artist) = facets.included_artists.first() {
			format!("/artist-{artist}-page-{}.html", page.max(1))
		} else if let Some(genre) = facets.included_genres.first() {
			format!("/genre-{genre}-page-{}.html", page.max(1))
		} else {
			format!("/new-porn-{}.html", page.max(1))
		};
		let html = self.request(&path)?.html()?;
		let (entries, raw_count) = self.parse_cards(&html, query.as_deref(), &facets);
		Ok(MangaPageResult {
			entries,
			has_next_page: query.as_ref().is_none_or(|q| q.trim().is_empty()) && raw_count >= 20,
		})
	}

	fn listing(id: &str, name: &str) -> Listing {
		Listing {
			id: id.into(),
			name: name.into(),
			kind: ListingKind::Default,
		}
	}

	fn facets_from_main(html: &Document) -> (FacetOptions, FacetOptions) {
		let mut genres: Vec<(String, String)> = Vec::new();
		if let Some(links) = html.select(".xcpreview a[href*='genre-']") {
			for link in links {
				let Some(id) = link
					.attr("href")
					.and_then(|href| Self::route_slug(&href, "genre"))
				else {
					continue;
				};
				let Some(label) = link
					.select_first(".xcpin")
					.and_then(|e| e.text())
					.or_else(|| link.text())
					.map(Self::clean)
				else {
					continue;
				};
				if !genres.iter().any(|(existing, _)| existing == &id) {
					genres.push((id, label.trim_start_matches('📚').trim().into()));
				}
			}
		}
		let mut artists: Vec<(String, String)> = Vec::new();
		if let Some(links) = html.select("a[href*='/artist-'][href*='-page-1.html']") {
			for link in links {
				let Some(id) = link
					.attr("href")
					.and_then(|href| Self::route_slug(&href, "artist"))
				else {
					continue;
				};
				let Some(label) = link.text().map(Self::clean) else {
					continue;
				};
				let label = label
					.trim_start_matches('🎨')
					.split('•')
					.next()
					.unwrap_or(&label)
					.trim()
					.into();
				if !artists.iter().any(|(existing, _)| existing == &id) {
					artists.push((id, label));
				}
			}
		}
		genres.sort_by_key(|(_, label)| label.to_ascii_lowercase());
		artists.sort_by_key(|(_, label)| label.to_ascii_lowercase());
		(genres, artists)
	}
}

impl Source for FreeComicsXxx {
	fn new() -> Self {
		set_rate_limit(3, 1, TimeUnit::Seconds);
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		self.search(query, page, filters)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let identity = Self::identity(&manga.key);
		let route = Self::route(&manga.key);
		let html = self.request(&route)?.html()?;
		if needs_details {
			manga.title = match identity {
				Identity::Series(_) => html.select_first(".xheadtitle, h1").and_then(|e| e.text()),
				Identity::Book(_) => html.select_first("title").and_then(|e| e.text()).map(|t| {
					t.split(" - FreeComics")
						.next()
						.unwrap_or(&t)
						.split("(Chapter")
						.next()
						.unwrap_or(&t)
						.trim()
						.into()
				}),
			}
			.map(Self::clean)
			.filter(|t| !t.is_empty())
			.unwrap_or(manga.title);
			manga.cover = html
				.select_first(".xcpreview img, meta[property='og:image'], .ximg")
				.and_then(|e| {
					e.attr("data-src")
						.or_else(|| e.attr("data-lazy-src"))
						.or_else(|| e.attr("data-original"))
						.or_else(|| e.attr("content"))
						.or_else(|| e.attr("abs:src"))
				})
				.or(manga.cover);
			if manga.cover.is_none() && matches!(identity, Identity::Book(_)) {
				let search = self
					.request(&format!("/?search={}", encode_uri_component(&manga.title)))?
					.html()?;
				manga.cover = self
					.parse_cards(&search, None, &SearchFacets::default())
					.0
					.into_iter()
					.find(|item| item.key == manga.key)
					.and_then(|item| item.cover);
			}
			manga.description = html
				.select_first("meta[property='og:description']")
				.and_then(|e| e.attr("content"));
			let artist = html
				.select_first(".xheadtitle a[href*='artist-'], a[href*='artist-']")
				.and_then(|e| e.text())
				.map(Self::clean)
				.filter(|v| !v.is_empty());
			manga.authors = artist.as_ref().map(|v| aidoku::alloc::vec![v.clone()]);
			manga.artists = artist.map(|v| aidoku::alloc::vec![v]);
			manga.tags = html.select("a[href*='genre-']").map(|els| {
				els.filter_map(|e| e.text())
					.map(Self::clean)
					.filter(|v| !v.is_empty())
					.collect()
			});
			manga.status = if matches!(identity, Identity::Series(_)) {
				MangaStatus::Ongoing
			} else {
				MangaStatus::Completed
			};
			manga.content_rating = ContentRating::NSFW;
			manga.viewer = Viewer::Webtoon;
			manga.update_strategy = if matches!(identity, Identity::Series(_)) {
				UpdateStrategy::Always
			} else {
				UpdateStrategy::Never
			};
			manga.url = Some(format!("{BASE_URL}{route}"));
			if needs_chapters {
				send_partial_result(&manga);
			}
		}
		if needs_chapters {
			let mut series_ids: Vec<String> = html
				.select(".xcpreview a")
				.map(|els| {
					els.filter_map(|e| {
						let href = Self::destination(&e.attr("href")?);
						Self::extract_between(&href, "/books/", ".html").map(String::from)
					})
					.collect()
				})
				.unwrap_or_default();
			series_ids.dedup();
			let chapter_html = match identity {
				Identity::Series(_) => {
					if let Some(first_id) = series_ids.first() {
						self.request(&format!("/books/{first_id}.html"))?.html()?
					} else {
						html
					}
				}
				Identity::Book(_) => html,
			};
			let mut dropdown_ids: Vec<String> = chapter_html
				.select(".dropdown-content a")
				.map(|els| {
					els.filter_map(|e| {
						let href = Self::destination(&e.attr("href")?);
						Self::extract_between(&href, "/books/", ".html").map(String::from)
					})
					.collect()
				})
				.unwrap_or_default();
			dropdown_ids.dedup();
			let mut ids = if dropdown_ids.is_empty() {
				series_ids
			} else {
				dropdown_ids
			};
			if ids.is_empty()
				&& let Identity::Book(id) = identity
			{
				ids.push(id.into());
			}
			ids.dedup();
			let len = ids.len();
			manga.chapters = Some(
				ids.into_iter()
					.enumerate()
					.map(|(index, key)| Chapter {
						key,
						title: Some(format!("Chapter {}", index + 1)),
						chapter_number: Some((index + 1) as f32),
						language: Some("en".into()),
						..Default::default()
					})
					.rev()
					.take(len)
					.collect(),
			);
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let referer = format!("{BASE_URL}/books/{}.html", chapter.key);
		let html = self.request(&referer)?.html()?;
		let mut urls: Vec<String> = html
			.select(".ximg")
			.map(|els| {
				els.filter_map(|e| {
					e.attr("data-src")
						.or_else(|| e.attr("data-lazy-src"))
						.or_else(|| e.attr("data-original"))
						.or_else(|| e.attr("abs:src"))
				})
				.filter(|url| url.contains("cdn.freecomics.xxx/galleries/"))
				.collect()
			})
			.unwrap_or_default();
		urls.dedup();
		if urls.is_empty() {
			bail!("Aucune page trouvée")
		}
		let mut context = PageContext::new();
		context.insert("Referer".into(), referer);
		Ok(urls
			.into_iter()
			.map(|url| Page {
				content: PageContent::url_context(url, context.clone()),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for FreeComicsXxx {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let path = match listing.id.as_str() {
			"popular" => format!("/popular-porn-{}.html", page.max(1)),
			"western" | "hentai" | "3d" => {
				format!("/genre-{}-page-{}.html", listing.id, page.max(1))
			}
			_ => format!("/new-porn-{}.html", page.max(1)),
		};
		let html = self.request(&path)?.html()?;
		let (entries, raw_count) = self.parse_cards(&html, None, &SearchFacets::default());
		Ok(MangaPageResult {
			entries,
			has_next_page: raw_count >= 20,
		})
	}
}

impl Home for FreeComicsXxx {
	fn get_home(&self) -> Result<HomeLayout> {
		let definitions = [
			("new", "Nouveaux comics", "/new-porn-1.html"),
			("popular", "Populaires", "/popular-porn-1.html"),
			("western", "Western", "/genre-western-page-1.html"),
			("hentai", "Hentai", "/genre-hentai-page-1.html"),
			("3d", "3D", "/genre-3d-page-1.html"),
		];
		let requests = definitions
			.iter()
			.map(|(_, _, path)| self.request(path))
			.collect::<Result<Vec<_>>>()?;
		let mut components = Vec::new();
		for ((id, title, _), response) in definitions.into_iter().zip(Request::send_all(requests)) {
			let Ok(html) = response.and_then(|response| response.get_html()) else {
				continue;
			};
			let (entries, _) = self.parse_cards(&html, None, &SearchFacets::default());
			if entries.is_empty() {
				continue;
			}
			components.push(HomeComponent {
				title: Some(title.into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: entries.into_iter().map(Into::into).collect(),
					listing: Some(Self::listing(id, title)),
				},
			});
		}
		Ok(HomeLayout { components })
	}
}

impl DynamicFilters for FreeComicsXxx {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let html = self.request("/main1.html")?.html()?;
		let (genres, artists) = Self::facets_from_main(&html);
		let (genre_ids, genre_labels): (Vec<Cow<'static, str>>, Vec<Cow<'static, str>>) = genres
			.into_iter()
			.map(|(id, label)| (Cow::Owned(id), Cow::Owned(label)))
			.unzip();
		let (artist_ids, artist_labels): (Vec<Cow<'static, str>>, Vec<Cow<'static, str>>) = artists
			.into_iter()
			.map(|(id, label)| (Cow::Owned(id), Cow::Owned(label)))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genre".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: true,
				uses_tag_style: true,
				options: genre_labels,
				ids: Some(genre_ids),
				..Default::default()
			}
			.into(),
			MultiSelectFilter {
				id: "artist".into(),
				title: Some("Artistes".into()),
				can_exclude: true,
				uses_tag_style: true,
				options: artist_labels,
				ids: Some(artist_ids),
				..Default::default()
			}
			.into(),
		])
	}
}

impl ImageRequestProvider for FreeComicsXxx {
	fn get_image_request(
		&self,
		url: String,
		context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		let referer = context
			.as_ref()
			.and_then(|context| context.get("Referer"))
			.map(String::as_str)
			.unwrap_or(BASE_URL);
		Ok(Request::get(url)?.header("Referer", referer))
	}
}

impl DeepLinkHandler for FreeComicsXxx {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if let Some(id) = Self::extract_between(&url, "/books/", ".html") {
			return Ok(Some(DeepLinkResult::Manga {
				key: format!("book--{id}"),
			}));
		}
		Ok(
			Self::extract_between(&url, "/series-", "-page-").map(|id| DeepLinkResult::Manga {
				key: format!("series--{id}"),
			}),
		)
	}
}

register_source!(
	FreeComicsXxx,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	ListingProvider
);
