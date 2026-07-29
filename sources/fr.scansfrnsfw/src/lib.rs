#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue,
	Home, HomeComponent, HomeComponentValue, HomeLayout, ImageRequestProvider, ImageResponse,
	Listing, ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, MultiSelectFilter,
	Page, PageContent, PageContext, PageImageProcessor, Result, SelectFilter, SortFilter, Source,
	Viewer,
	alloc::{String, Vec, borrow::Cow, format, string::ToString, vec},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{
		canvas::ImageRef,
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

fn reader_url(url: &str) -> String {
	format!("{}#aidoku-v6", url.split('#').next().unwrap_or(url))
}
const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

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
			.header("Referer", &format!("{BASE_URL}/nsfw"))
			.header("User-Agent", USER_AGENT)
			.header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8"))
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
			.attr("data-src")
			.or_else(|| image.attr("data-lazy-src"))
			.or_else(|| image.attr("abs:src"))
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
			url: Some(if href.starts_with("http") {
				href
			} else {
				format!("{BASE_URL}{href}")
			}),
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
							url: Some(format!("{BASE_URL}/nsfw/read/{}/{number}", manga.key)),
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
		let chapter_number = chapter.chapter_number.unwrap_or_default();
		let referer = chapter
			.url
			.clone()
			.unwrap_or_else(|| format!("{BASE_URL}/nsfw/read/{}/{chapter_number}", manga.key));
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
		.header("Referer", &referer)
		.header("User-Agent", USER_AGENT)
		.header("Content-Type", "application/json")
		.body(body)
		.json_owned::<TokenJson>()?;
		let count = if token.page_count > 0 {
			token.page_count
		} else {
			chapter_data.page_count.unwrap_or_default()
		};
		let mut context = PageContext::new();
		context.insert("Referer".into(), referer);
		context.insert("Cookie".into(), COOKIE.into());
		Ok((1..=count)
			.map(|index| Page {
				content: PageContent::url_context(
					format!(
						"{API_URL}/api/v1/images/{}/{index}?sig={}&exp={}&s={}",
						token.chapter_id,
						encode_uri_component(&token.sig),
						token.exp,
						encode_uri_component(&token.session_hash)
					),
					context.clone(),
				),
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
			("popular", "À la une", "popular"),
			("updated", "Dernières sorties", "updated"),
			("latest", "Nouveautés", "latest"),
			("rating", "Les mieux notés", "rating"),
		];
		let requests = definitions
			.iter()
			.map(|(sort, _, _)| {
				self.request(format!(
					"{BASE_URL}/nsfw/catalog?page=1&limit=24&sort={sort}"
				))
			})
			.collect::<Result<Vec<_>>>()?;
		let mut components = Vec::new();
		for ((_, title, listing_id), response) in
			definitions.into_iter().zip(Request::send_all(requests))
		{
			let Ok(response) = response else { continue };
			let Ok(html) = response.get_html() else {
				continue;
			};
			let entries = self.parse_catalog(&html).entries;
			if entries.is_empty() {
				continue;
			}
			components.push(HomeComponent {
				title: Some(title.into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: entries.into_iter().map(Into::into).collect(),
					listing: Some(Self::listing(listing_id, title)),
				},
			});
		}
		Ok(HomeLayout { components })
	}
}

impl DynamicFilters for ScansFrNsfw {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let html = self
			.request(format!("{BASE_URL}/nsfw/catalog"))
			.ok()
			.and_then(|request| request.html().ok());
		let (mut type_options, mut type_ids) = html
			.as_ref()
			.map(|html| Self::options_for(html, "manga"))
			.unwrap_or_default();
		let (mut genre_options, mut genre_ids) = html
			.as_ref()
			.map(|html| Self::options_for(html, "Hentai"))
			.unwrap_or_default();
		let (mut status_options, mut status_ids) = html
			.as_ref()
			.map(|html| Self::options_for(html, "ongoing"))
			.unwrap_or_default();
		if type_options.is_empty() {
			type_options = vec![
				"Tous types".into(),
				"Manga".into(),
				"Manhwa".into(),
				"Manhua".into(),
				"Webtoon".into(),
			];
			type_ids = vec![
				"".into(),
				"manga".into(),
				"manhwa".into(),
				"manhua".into(),
				"webtoon".into(),
			];
		}
		if genre_options.is_empty() {
			genre_options = vec![
				"Boy's Love".into(),
				"Ecchi".into(),
				"Harem".into(),
				"Hentai".into(),
				"Mature".into(),
				"Pornhwa".into(),
				"Smut".into(),
				"Yuri".into(),
			];
			genre_ids = vec![
				"Boy's Love".into(),
				"Ecchi".into(),
				"Harem".into(),
				"Hentai".into(),
				"Mature".into(),
				"Pornhwa".into(),
				"Smut".into(),
				"Yuri".into(),
			];
		}
		if status_options.is_empty() {
			status_options = vec![
				"Tous".into(),
				"En cours".into(),
				"Terminé".into(),
				"En pause".into(),
				"Abandonné".into(),
			];
			status_ids = vec![
				"".into(),
				"ongoing".into(),
				"completed".into(),
				"hiatus".into(),
				"cancelled".into(),
			];
		}
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
		context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		let referer = context
			.as_ref()
			.and_then(|context| context.get("Referer"))
			.map(String::as_str)
			.unwrap_or(BASE_URL);
		let cookie = context
			.as_ref()
			.and_then(|context| context.get("Cookie"))
			.map(String::as_str)
			.unwrap_or(COOKIE);
		Ok(Request::get(reader_url(&url))?
			.header("Cookie", cookie)
			.header("Referer", referer)
			.header("User-Agent", USER_AGENT)
			.header("Cache-Control", "no-cache")
			.header("Pragma", "no-cache")
			.header(
				"Accept",
				"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
			)
			.header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8"))
	}
}

impl PageImageProcessor for ScansFrNsfw {
	fn process_page_image(
		&self,
		response: ImageResponse,
		context: Option<PageContext>,
	) -> Result<ImageRef> {
		if response.code < 400 {
			return Ok(response.image);
		}
		let url = response
			.request
			.url
			.ok_or_else(|| error!("URL d’image manquante"))?;
		Ok(self.get_image_request(url, context)?.image()?)
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
	PageImageProcessor,
	ListingProvider
);

#[cfg(test)]
mod test {
	use super::*;
	use aidoku_test::aidoku_test;

	#[aidoku_test]
	fn reader_returns_a_decodable_image() {
		let source = ScansFrNsfw::new();
		let manga = source
			.search(None, 1, Vec::new())
			.unwrap()
			.entries
			.into_iter()
			.next()
			.unwrap();
		let manga = source.get_manga_update(manga, false, true).unwrap();
		let chapter = manga.chapters.clone().unwrap().into_iter().next().unwrap();
		let mut pages = source.get_page_list(manga, chapter).unwrap();
		assert!(!pages.is_empty());
		let PageContent::Url(url, context) = pages.remove(0).content else {
			panic!("La première page n'est pas une URL")
		};
		let image = source
			.get_image_request(url, context)
			.unwrap()
			.image()
			.unwrap();
		assert!(image.width() > 0.0 && image.height() > 0.0);
	}

	#[aidoku_test]
	fn home_and_filters_are_populated() {
		let source = ScansFrNsfw::new();
		assert!(source.get_home().unwrap().components.len() >= 3);
		assert!(source.get_dynamic_filters().unwrap().len() >= 5);
	}
}
