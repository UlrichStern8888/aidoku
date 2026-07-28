#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue,
	Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, ImageRequestProvider,
	Listing, ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, MultiSelectFilter,
	Page, PageContent, Result, SelectFilter, SortFilter, Source, Viewer,
	alloc::{String, Vec, borrow::Cow, format, string::ToString, vec},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{
		html::{Document, Element},
		net::{Request, TimeUnit, set_rate_limit},
		std::{current_date, send_partial_result},
	},
	prelude::*,
};
use chrono::DateTime;
use serde::Deserialize;

const BASE_URL: &str = "https://scansfr.com";
const API_URL: &str = "https://api.scansfr.com";
const COOKIE: &str = "scansfr_age_verified=true";

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MangaJson {
	title: String,
	description: Option<String>,
	cover: String,
	status: Option<String>,
	tags: Option<Vec<String>>,
	author: Option<String>,
	artist: Option<String>,
	is_nsfw: bool,
	chapters_list: Option<Vec<ChapterJson>>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChapterJson {
	id: Option<String>,
	number: Option<f32>,
	title: Option<String>,
	date: Option<String>,
	is_early_access: Option<bool>,
	page_count: Option<usize>,
	manga_is_nsfw: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenJson {
	sig: String,
	exp: i64,
	session_hash: String,
	chapter_id: String,
	page_count: usize,
}

struct ScansFrNsfw;

impl ScansFrNsfw {
	fn request<T: AsRef<str>>(&self, url: T) -> Result<Request> {
		Ok(Request::get(url)?
			.header("Cookie", COOKIE)
			.header("Referer", &format!("{BASE_URL}/nsfw")))
	}

	fn manga_json(&self, key: &str) -> Result<MangaJson> {
		self.request(format!(
			"{API_URL}/api/v1/mangas/{}",
			encode_uri_component(key)
		))?
		.json_owned::<MangaJson>()
	}

	fn image_url(path: &str) -> String {
		if path.starts_with("http") {
			path.into()
		} else {
			format!("{API_URL}{path}")
		}
	}

	fn key_from_href(href: &str) -> Option<String> {
		let path = href.split('?').next().unwrap_or(href).trim_end_matches('/');
		path.rsplit('/')
			.next()
			.filter(|key| !key.is_empty())
			.map(String::from)
	}

	fn parse_catalog_link(element: Element) -> Option<Manga> {
		let href = element.attr("href")?;
		let key = Self::key_from_href(&href)?;
		let image = element.select_first("img")?;
		let cover = image
			.attr("abs:src")
			.or_else(|| image.attr("data-src"))
			.map(|url| Self::image_url(&url));
		let title = image
			.attr("alt")
			.or_else(|| element.select_first("h2, h3, p").and_then(|e| e.text()))?
			.trim_start_matches("Couverture de ")
			.trim()
			.into();
		Some(Manga {
			key,
			title,
			cover,
			content_rating: ContentRating::NSFW,
			viewer: Viewer::Webtoon,
			..Default::default()
		})
	}

	fn deduplicate(entries: impl Iterator<Item = Manga>) -> Vec<Manga> {
		let mut result: Vec<Manga> = Vec::new();
		for manga in entries {
			if !result.iter().any(|item| item.key == manga.key) {
				result.push(manga);
			}
		}
		result
	}

	fn parse_catalog(&self, html: &Document) -> MangaPageResult {
		let entries = html
			.select("a[href^='/nsfw/manga/'], a[href^=\"/nsfw/manga/\"]")
			.map(|els| Self::deduplicate(els.filter_map(Self::parse_catalog_link)))
			.unwrap_or_default();
		let has_next_page = html
			.select("button")
			.and_then(|mut buttons| {
				buttons.find(|button| {
					button.text().is_some_and(|text| text.trim() == "Suivant")
						&& button.attr("disabled").is_none()
				})
			})
			.is_some();
		MangaPageResult {
			entries,
			has_next_page,
		}
	}

	fn search(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		qs.push("page", Some(&page.max(1).to_string()));
		qs.push("limit", Some("24"));
		if let Some(query) = query.as_deref().filter(|q| !q.trim().is_empty()) {
			qs.push("search", Some(query.trim()));
		}
		for filter in filters {
			match filter {
				FilterValue::Select { id, value } if !value.is_empty() => {
					qs.push(&id, Some(&value))
				}
				FilterValue::Sort { index, .. } => qs.push(
					"sort",
					Some(match index {
						1 => "latest",
						2 => "updated",
						3 => "rating",
						4 => "alphabetical",
						_ => "popular",
					}),
				),
				FilterValue::MultiSelect { id, included, .. } if !included.is_empty() => {
					qs.push(&id, Some(&included.join(",")))
				}
				_ => {}
			}
		}
		let html = self
			.request(format!("{BASE_URL}/nsfw/catalog?{qs}"))?
			.html()?;
		Ok(self.parse_catalog(&html))
	}

	fn listing(id: &str, title: &str) -> Listing {
		Listing {
			id: id.into(),
			name: title.into(),
			kind: ListingKind::Default,
		}
	}

	fn sort_filter(index: i32) -> Vec<FilterValue> {
		vec![FilterValue::Sort {
			id: "sort".into(),
			index,
			ascending: false,
		}]
	}

	fn options_for(
		html: &Document,
		known_value: &str,
	) -> (Vec<Cow<'static, str>>, Vec<Cow<'static, str>>) {
		let Some(select) = html.select("select").and_then(|mut selects| {
			selects.find(|select| {
				select.select("option").is_some_and(|options| {
					options.into_iter().any(|option| {
						option
							.attr("value")
							.is_some_and(|value| value == known_value)
					})
				})
			})
		}) else {
			return (Vec::new(), Vec::new());
		};
		select
			.select("option")
			.map(|options| {
				options
					.filter_map(|option| {
						let id = option.attr("value")?;
						if id.is_empty() {
							return None;
						}
						Some((Cow::Owned(option.text()?.trim().into()), Cow::Owned(id)))
					})
					.unzip()
			})
			.unwrap_or_default()
	}
}

impl Source for ScansFrNsfw {
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
		let data = self.manga_json(&manga.key)?;
		if !data.is_nsfw {
			bail!("Titre hors du catalogue /nsfw")
		}
		if needs_details {
			manga.title = data.title;
			manga.cover = Some(Self::image_url(&data.cover));
			manga.description = data.description;
			manga.authors = data.author.map(|v| vec![v]);
			manga.artists = data.artist.map(|v| vec![v]);
			manga.tags = data.tags;
			manga.status = match data
				.status
				.as_deref()
				.unwrap_or_default()
				.to_ascii_lowercase()
				.as_str()
			{
				"ongoing" | "en cours" => MangaStatus::Ongoing,
				"completed" | "terminé" => MangaStatus::Completed,
				"hiatus" | "en pause" => MangaStatus::Hiatus,
				"cancelled" | "annulé" => MangaStatus::Cancelled,
				_ => MangaStatus::Unknown,
			};
			manga.content_rating = ContentRating::NSFW;
			manga.viewer = Viewer::Webtoon;
			manga.url = Some(format!("{BASE_URL}/nsfw/manga/{}", manga.key));
			if needs_chapters {
				send_partial_result(&manga);
			}
		}
		if needs_chapters {
			manga.chapters = Some(
				data.chapters_list
					.unwrap_or_default()
					.into_iter()
					.filter(|c| !c.is_early_access.unwrap_or(false))
					.map(|c| {
						let number = c.number.unwrap_or_default();
						Chapter {
							key: c.id.unwrap_or_else(|| number.to_string()),
							title: c.title.or_else(|| Some(format!("Chapitre {number}"))),
							chapter_number: Some(number),
							date_uploaded: c
								.date
								.and_then(|d| DateTime::parse_from_rfc3339(&d).ok())
								.map(|d| d.timestamp()),
							language: Some("fr".into()),
							..Default::default()
						}
					})
					.collect(),
			);
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let key = format!("{}-{}", manga.key, chapter.key);
		let chapter_data = self
			.request(format!(
				"{API_URL}/api/v1/chapters/{}",
				encode_uri_component(&key)
			))?
			.json_owned::<ChapterJson>()?;
		if !chapter_data.manga_is_nsfw.unwrap_or(false) {
			bail!("Chapitre hors du catalogue /nsfw")
		}
		let body = format!("{{\"sessionId\":\"aidoku_{}\"}}", current_date());
		let token = Request::post(format!(
			"{API_URL}/api/v1/chapters/{}/token",
			encode_uri_component(&key)
		))?
		.header("Cookie", COOKIE)
		.header("Referer", &format!("{BASE_URL}/nsfw"))
		.header("Content-Type", "application/json")
		.body(body)
		.json_owned::<TokenJson>()?;
		let count = if token.page_count > 0 {
			token.page_count
		} else {
			chapter_data.page_count.unwrap_or_default()
		};
		Ok((1..=count)
			.map(|index| Page {
				content: PageContent::url(format!(
					"{API_URL}/api/v1/images/{}/{index}?sig={}&exp={}&s={}",
					token.chapter_id,
					encode_uri_component(&token.sig),
					token.exp,
					encode_uri_component(&token.session_hash)
				)),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for ScansFrNsfw {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let index = match listing.id.as_str() {
			"latest" => 1,
			"updated" => 2,
			"rating" => 3,
			"alphabetical" => 4,
			_ => 0,
		};
		self.search(None, page, Self::sort_filter(index))
	}
}

impl Home for ScansFrNsfw {
	fn get_home(&self) -> Result<HomeLayout> {
		let definitions = [
			("featured", "À la une", "", "featured"),
			(
				"updated",
				"Dernières sorties",
				"Dernieres Sorties",
				"updated",
			),
			("latest", "Nouveautés", "Nouveautes", "latest"),
			("views", "Top", "Top", "popular"),
		];
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: definitions
				.iter()
				.map(|(_, title, _, _)| HomeComponent {
					title: Some((*title).into()),
					subtitle: None,
					value: HomeComponentValue::empty_scroller(),
				})
				.collect(),
		}));
		let html = self.request(format!("{BASE_URL}/nsfw"))?.html()?;
		for (id, title, heading, listing_id) in definitions {
			let entries = if id == "featured" {
				html.select("a[href^='/nsfw/manga/']")
					.and_then(|mut links| links.find_map(Self::parse_catalog_link))
					.into_iter()
					.collect::<Vec<_>>()
			} else {
				html.select("section")
					.and_then(|mut sections| {
						sections.find(|section| {
							section
								.select_first("h1, h2, h3")
								.and_then(|h| h.text())
								.is_some_and(|text| text.trim().eq_ignore_ascii_case(heading))
						})
					})
					.and_then(|section| section.select("a[href^='/nsfw/manga/']"))
					.map(|links| Self::deduplicate(links.filter_map(Self::parse_catalog_link)))
					.unwrap_or_default()
			};
			if entries.is_empty() {
				continue;
			}
			send_partial_result(&HomePartialResult::Component(HomeComponent {
				title: Some(title.into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: entries.into_iter().map(Into::into).collect(),
					listing: Some(Self::listing(listing_id, title)),
				},
			}));
		}
		Ok(HomeLayout::default())
	}
}

impl DynamicFilters for ScansFrNsfw {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let html = self.request(format!("{BASE_URL}/nsfw/catalog"))?.html()?;
		let (type_options, type_ids) = Self::options_for(&html, "manga");
		let (genre_options, genre_ids) = Self::options_for(&html, "Hentai");
		let (status_options, status_ids) = Self::options_for(&html, "ongoing");
		Ok(vec![
			SelectFilter {
				id: "type".into(),
				title: Some("Type".into()),
				options: type_options,
				ids: Some(type_ids),
				..Default::default()
			}
			.into(),
			MultiSelectFilter {
				id: "genre".into(),
				title: Some("Genres NSFW".into()),
				is_genre: true,
				can_exclude: false,
				uses_tag_style: true,
				options: genre_options,
				ids: Some(genre_ids),
				..Default::default()
			}
			.into(),
			SelectFilter {
				id: "status".into(),
				title: Some("Statut".into()),
				options: status_options,
				ids: Some(status_ids),
				..Default::default()
			}
			.into(),
			SelectFilter {
				id: "minChapters".into(),
				title: Some("Chapitres minimum".into()),
				options: vec![
					"Tous".into(),
					"10+".into(),
					"25+".into(),
					"50+".into(),
					"100+".into(),
					"200+".into(),
				],
				ids: Some(vec![
					"".into(),
					"10".into(),
					"25".into(),
					"50".into(),
					"100".into(),
					"200".into(),
				]),
				..Default::default()
			}
			.into(),
			SortFilter {
				id: "sort".into(),
				title: Some("Tri".into()),
				can_ascend: false,
				options: vec![
					"Popularité".into(),
					"Nouveautés".into(),
					"Mises à jour".into(),
					"Note".into(),
					"Ordre alphabétique".into(),
				],
				..Default::default()
			}
			.into(),
		])
	}
}

impl ImageRequestProvider for ScansFrNsfw {
	fn get_image_request(
		&self,
		url: String,
		_context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		self.request(url)
	}
}

impl DeepLinkHandler for ScansFrNsfw {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(url
			.split("/nsfw/manga/")
			.nth(1)
			.and_then(Self::key_from_href)
			.map(|key| DeepLinkResult::Manga { key }))
	}
}

register_source!(
	ScansFrNsfw,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	ListingProvider
);
