#![no_std]
//! Native Aidoku source for OrtegaScans and its JSON/HTML endpoints.

use aidoku::{
	Chapter, CheckFilter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter,
	FilterValue, Home, HomeComponent, HomeComponentValue, HomeLayout, ImageRequestProvider,
	ImageResponse, Listing, ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus,
	MultiSelectFilter, Page, PageContent, PageContext, PageImageProcessor, Result, SelectFilter,
	SortFilter, Source, TextFilter, Viewer,
	alloc::{String, Vec, borrow::Cow, format, string::ToString, vec},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{
		canvas::ImageRef,
		html::{Document, Element, Html},
		net::{Request, TimeUnit, set_rate_limit},
		std::send_partial_result,
	},
	prelude::*,
};
use regex::Regex;
use serde::Deserialize;

const BASE_URL: &str = "https://ortegascans.fr";

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Category {
	name: String,
}

#[derive(Default, Deserialize)]
struct ChapterCount {
	chapters: Option<usize>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesJson {
	slug: String,
	title: String,
	status: String,
	categories: Option<Vec<Category>>,
	#[serde(rename = "_count")]
	count: Option<ChapterCount>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesResponse {
	success: bool,
	data: Vec<SeriesJson>,
	has_more: bool,
}

struct OrtegaScans;

impl OrtegaScans {
	fn cover_url(key: &str) -> String {
		format!("{BASE_URL}/api/covers/{}.webp", encode_uri_component(key))
	}

	fn get_series_page(&self, query: QueryParameters) -> Result<SeriesResponse> {
		let payload = Request::get(format!("{BASE_URL}/api/series?{query}"))?
			.json_owned::<SeriesResponse>()?;
		if !payload.success {
			bail!("Réponse invalide de l'API OrtegaScans")
		}
		Ok(payload)
	}

	fn series_to_manga(series: SeriesJson) -> Manga {
		let mut tags: Vec<String> = series
			.categories
			.unwrap_or_default()
			.into_iter()
			.map(|c| c.name)
			.collect();
		if let Some(count) = series.count.and_then(|c| c.chapters) {
			tags.push(format!("{count} chapitres"));
		}
		Manga {
			key: series.slug.clone(),
			title: series.title,
			cover: Some(Self::cover_url(&series.slug)),
			tags: Some(tags),
			status: map_status(&series.status),
			content_rating: ContentRating::NSFW,
			viewer: Viewer::Webtoon,
			url: Some(format!("{BASE_URL}/serie/{}", series.slug)),
			..Default::default()
		}
	}

	fn search(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		qs.push("limit", Some("18"));
		qs.push("page", Some(&page.max(1).to_string()));
		qs.push("minChapters", Some("0"));
		qs.push("isOrtegaOnly", Some("false"));
		qs.push("unreadOnly", Some("false"));
		qs.push("sort", Some("popular"));
		if let Some(query) = query.as_deref().filter(|q| !q.trim().is_empty()) {
			qs.push("search", Some(query.trim()));
		}
		for filter in filters {
			match filter {
				FilterValue::Text { id, value } if id == "tags" && !value.is_empty() => {
					qs.set("tags", Some(&value))
				}
				FilterValue::Select { id, value } if !value.is_empty() => qs.set(&id, Some(&value)),
				FilterValue::Check { id, value } if id == "isOrtegaOnly" => qs.set(
					"isOrtegaOnly",
					Some(if value == 1 { "true" } else { "false" }),
				),
				FilterValue::Sort { index, .. } => qs.set(
					"sort",
					Some(match index {
						1 => "alpha",
						2 => "recent",
						_ => "popular",
					}),
				),
				FilterValue::MultiSelect { id, included, .. } if !included.is_empty() => qs.set(
					if id == "genres" { "tags" } else { &id },
					Some(&included.join(",")),
				),
				_ => {}
			}
		}
		let payload = self.get_series_page(qs)?;
		Ok(MangaPageResult {
			entries: payload
				.data
				.into_iter()
				.map(Self::series_to_manga)
				.collect(),
			has_next_page: payload.has_more,
		})
	}

	fn parse_series_link(element: Element) -> Option<Manga> {
		let href = element.attr("href")?;
		if href.contains("/chapter/") {
			return None;
		}
		let key: String = href
			.split("/serie/")
			.nth(1)?
			.trim_matches('/')
			.split('?')
			.next()?
			.into();
		let image = element.select_first("img");
		let title = element
			.select_first("h3")
			.and_then(|e| e.text())
			.or_else(|| image.as_ref().and_then(|e| e.attr("alt")))
			.or_else(|| element.text())?
			.trim()
			.into();
		let cover = Some(Self::cover_url(&key));
		Some(Manga {
			url: Some(format!("{BASE_URL}/serie/{key}")),
			key,
			title,
			cover,
			content_rating: ContentRating::NSFW,
			viewer: Viewer::Webtoon,
			..Default::default()
		})
	}

	fn parse_section(html: &Document, heading: &str) -> Vec<Manga> {
		let mut entries = Vec::new();
		let links = html
			.select("section")
			.and_then(|mut sections| {
				sections.find(|section| {
					section
						.select_first("h1, h2, h3")
						.and_then(|h| h.text())
						.is_some_and(|text| text.trim().eq_ignore_ascii_case(heading))
				})
			})
			.and_then(|section| section.select("a[href^='/serie/']"));
		if let Some(links) = links {
			for manga in links.filter_map(Self::parse_series_link) {
				if !entries.iter().any(|item: &Manga| item.key == manga.key) {
					entries.push(manga);
				}
			}
		}
		entries
	}

	fn listing(id: &str, name: &str) -> Listing {
		Listing {
			id: id.into(),
			name: name.into(),
			kind: ListingKind::Default,
		}
	}
}

fn map_status(status: &str) -> MangaStatus {
	match status.to_ascii_lowercase().as_str() {
		"en cours" | "ongoing" => MangaStatus::Ongoing,
		"terminé" | "termine" | "completed" => MangaStatus::Completed,
		"en pause" | "hiatus" => MangaStatus::Hiatus,
		"annulé" | "annule" | "cancelled" => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

impl Source for OrtegaScans {
	fn new() -> Self {
		set_rate_limit(4, 1, TimeUnit::Seconds);
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
		let url = format!("{BASE_URL}/serie/{}", manga.key);
		let raw = Request::get(&url)?.string()?;
		let html = Html::parse_with_url(&raw, &url)?;
		if needs_details {
			manga.title = html
				.select_first("h1")
				.and_then(|e| e.text())
				.unwrap_or(manga.title);
			// Ortega exposes a stable cover endpoint. Selecting the first image in
			// the page is unsafe because the site logo precedes the actual cover.
			manga.cover = Some(Self::cover_url(&manga.key));
			manga.description = html
				.select_first("meta[name=description]")
				.and_then(|e| e.attr("content"));
			manga.tags = html.select("a[href*='genre'], a[href*='tag']").map(|els| {
				els.filter_map(|e| e.text())
					.filter(|s| !s.is_empty())
					.collect()
			});
			let main_text = html
				.select_first("main")
				.and_then(|e| e.text())
				.unwrap_or_default();
			manga.authors = extract_labeled(&main_text, "Auteur").map(|v| vec![v]);
			manga.artists = extract_labeled(&main_text, "Artiste").map(|v| vec![v]);
			manga.status = extract_labeled(&main_text, "Statut")
				.or_else(|| extract_labeled(&main_text, "Status"))
				.map(|s| map_status(&s))
				.unwrap_or_default();
			manga.content_rating = ContentRating::NSFW;
			manga.viewer = Viewer::Webtoon;
			manga.url = Some(url.clone());
			if needs_chapters {
				send_partial_result(&manga);
			}
		}
		if needs_chapters {
			let mut chapters: Vec<Chapter> = html
				.select("a[href*='/chapter/']")
				.map(|els| {
					els.filter_map(|e| {
						let href = e.attr("href")?;
						if e.text()
							.unwrap_or_default()
							.to_ascii_uppercase()
							.contains("PREMIUM")
						{
							return None;
						}
						let key: String = href
							.split("/chapter/")
							.nth(1)?
							.trim_matches('/')
							.split('?')
							.next()?
							.into();
						let number = key.parse::<f32>().ok();
						let chapter_url =
							if href.starts_with("http://") || href.starts_with("https://") {
								href.clone()
							} else {
								format!("{BASE_URL}{href}")
							};
						Some(Chapter {
							key,
							title: number.map(|n| format!("Chapitre {n}")),
							chapter_number: number,
							language: Some("fr".into()),
							url: Some(chapter_url),
							..Default::default()
						})
					})
					.collect()
				})
				.unwrap_or_default();
			let list_regex = Regex::new(r#"(?s)\\?\"chapters\\?\":\[(.*?)\],\\?\"_count"#)
				.map_err(|e| error!("Regex invalide: {e}"))?;
			let object_regex =
				Regex::new(r#"\{(.*?)\}"#).map_err(|e| error!("Regex invalide: {e}"))?;
			let number_regex = Regex::new(r#"\\?\"number\\?\":(\d+(?:\.\d+)?)"#)
				.map_err(|e| error!("Regex invalide: {e}"))?;
			if let Some(list) = list_regex.captures(&raw).and_then(|c| c.get(1)) {
				for object in object_regex
					.captures_iter(list.as_str())
					.filter_map(|c| c.get(1))
				{
					if object.as_str().contains(r#"\"isPremium\":true"#)
						|| object.as_str().contains(r#"\\\"isPremium\\\":true"#)
					{
						continue;
					}
					let Some(number) = number_regex
						.captures(object.as_str())
						.and_then(|c| c.get(1))
						.and_then(|m| m.as_str().parse::<f32>().ok())
					else {
						continue;
					};
					let key = number.to_string();
					if !chapters.iter().any(|chapter| chapter.key == key) {
						chapters.push(Chapter {
							url: Some(format!("{BASE_URL}/serie/{}/chapter/{key}", manga.key)),
							key,
							title: Some(format!("Chapitre {number}")),
							chapter_number: Some(number),
							language: Some("fr".into()),
							..Default::default()
						});
					}
				}
			}
			chapters.sort_by(|a, b| {
				b.chapter_number
					.partial_cmp(&a.chapter_number)
					.unwrap_or(core::cmp::Ordering::Equal)
			});
			chapters.dedup_by(|a, b| a.key == b.key);
			manga.chapters = Some(chapters);
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let referer = format!("{BASE_URL}/serie/{}/chapter/{}", manga.key, chapter.key);
		let raw = Request::get(&referer)?.string()?;
		let regex =
			Regex::new(r#"\\?\"url\\?\":\\?\"(/api/chapters/[^\"\\]+/image/[^\"\\]+)\\?\""#)
				.map_err(|e| error!("Regex invalide: {e}"))?;
		let mut urls: Vec<String> = regex
			.captures_iter(&raw)
			.filter_map(|c| c.get(1).map(|m| format!("{BASE_URL}{}", m.as_str())))
			.collect();
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

impl ListingProvider for OrtegaScans {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let sort_index = if listing.id == "popular" { 0 } else { 2 };
		self.search(
			None,
			page,
			vec![FilterValue::Sort {
				id: "sort".into(),
				index: sort_index,
				ascending: false,
			}],
		)
	}
}

impl Home for OrtegaScans {
	fn get_home(&self) -> Result<HomeLayout> {
		let mut components = Vec::new();
		let mut qs = QueryParameters::new();
		qs.push("limit", Some("18"));
		qs.push("page", Some("1"));
		qs.push("sort", Some("popular"));
		qs.push("minChapters", Some("0"));
		qs.push("isOrtegaOnly", Some("false"));
		qs.push("unreadOnly", Some("false"));
		let mut responses = Request::send_all([
			Request::get(BASE_URL)?,
			Request::get(format!("{BASE_URL}/api/series?{qs}"))?,
		])
		.into_iter();
		if let Some(Ok(response)) = responses.next()
			&& let Ok(html) = response.get_html()
		{
			for (id, title) in [("latest", "Dernières sorties"), ("new", "Nouvelles séries")] {
				let entries = Self::parse_section(&html, title);
				if !entries.is_empty() {
					components.push(HomeComponent {
						title: Some(title.into()),
						subtitle: None,
						value: HomeComponentValue::Scroller {
							entries: entries.into_iter().map(Into::into).collect(),
							listing: Some(Self::listing(id, title)),
						},
					});
				}
			}
		}
		if let Some(Ok(response)) = responses.next()
			&& let Ok(payload) = response.get_json_owned::<SeriesResponse>()
			&& payload.success
		{
			components.push(HomeComponent {
				title: Some("Séries populaires".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: payload
						.data
						.into_iter()
						.map(Self::series_to_manga)
						.map(Into::into)
						.collect(),
					listing: Some(Self::listing("popular", "Séries populaires")),
				},
			});
		}
		Ok(HomeLayout { components })
	}
}

impl DynamicFilters for OrtegaScans {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let mut qs = QueryParameters::new();
		qs.push("limit", Some("1000"));
		qs.push("page", Some("1"));
		qs.push("sort", Some("popular"));
		qs.push("minChapters", Some("0"));
		qs.push("isOrtegaOnly", Some("false"));
		qs.push("unreadOnly", Some("false"));
		let payload = self.get_series_page(qs)?;
		let mut genres: Vec<String> = Vec::new();
		for genre in payload
			.data
			.into_iter()
			.flat_map(|series| series.categories.unwrap_or_default())
			.map(|category| category.name)
		{
			if !genres
				.iter()
				.any(|existing| existing.eq_ignore_ascii_case(&genre))
			{
				genres.push(genre);
			}
		}
		genres.sort_by_key(|value| value.to_ascii_lowercase());
		let genre_options: Vec<Cow<'static, str>> =
			genres.iter().cloned().map(Cow::Owned).collect();
		let genre_ids: Vec<Cow<'static, str>> = genres.into_iter().map(Cow::Owned).collect();
		Ok(vec![
			TextFilter {
				id: "tags".into(),
				title: Some("Tags".into()),
				placeholder: Some("Ex. romance, mature…".into()),
				..Default::default()
			}
			.into(),
			MultiSelectFilter {
				id: "genres".into(),
				title: Some("Genres".into()),
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
				options: vec![
					"Tous".into(),
					"En cours".into(),
					"Terminé".into(),
					"En pause".into(),
					"Annulé".into(),
				],
				ids: Some(vec![
					"".into(),
					"en cours".into(),
					"terminé".into(),
					"en pause".into(),
					"annulé".into(),
				]),
				..Default::default()
			}
			.into(),
			SelectFilter {
				id: "minChapters".into(),
				title: Some("Chapitres minimum".into()),
				options: vec![
					"Tous".into(),
					"1+".into(),
					"25+".into(),
					"50+".into(),
					"100+".into(),
					"150+".into(),
					"200+".into(),
				],
				ids: Some(vec![
					"0".into(),
					"1".into(),
					"25".into(),
					"50".into(),
					"100".into(),
					"150".into(),
					"200".into(),
				]),
				..Default::default()
			}
			.into(),
			CheckFilter {
				id: "isOrtegaOnly".into(),
				title: Some("Catalogue".into()),
				name: Some("Séries Ortega uniquement".into()),
				..Default::default()
			}
			.into(),
			SortFilter {
				id: "sort".into(),
				title: Some("Tri".into()),
				can_ascend: false,
				options: vec![
					"Popularité".into(),
					"Ordre alphabétique".into(),
					"Plus récent".into(),
				],
				..Default::default()
			}
			.into(),
		])
	}
}

fn extract_labeled(text: &str, label: &str) -> Option<String> {
	let rest = text
		.split_once(label)?
		.1
		.trim_start_matches([' ', ':'])
		.trim();
	let end = [
		"Auteur",
		"Artiste",
		"Année de sortie",
		"Tags",
		"Type",
		"Status",
		"Statut",
		"Chapitres",
	]
	.into_iter()
	.filter_map(|next| rest.find(next))
	.min()
	.unwrap_or(rest.len());
	let value = rest[..end].trim();
	(!value.is_empty()).then(|| value.into())
}

impl ImageRequestProvider for OrtegaScans {
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
		Ok(Request::get(url)?
			.header("Referer", referer)
			.header("Cache-Control", "no-cache")
			.header("Pragma", "no-cache")
			.header(
				"Accept",
				"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
			)
			.header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8"))
	}
}

impl PageImageProcessor for OrtegaScans {
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

impl DeepLinkHandler for OrtegaScans {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.split("/serie/").nth(1) else {
			return Ok(None);
		};
		let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
		Ok(match parts.as_slice() {
			[manga, "chapter", chapter, ..] => Some(DeepLinkResult::Chapter {
				manga_key: (*manga).into(),
				key: (*chapter).into(),
			}),
			[manga, ..] => Some(DeepLinkResult::Manga {
				key: (*manga).into(),
			}),
			_ => None,
		})
	}
}

register_source!(
	OrtegaScans,
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
	fn reader_returns_jpeg_data() {
		let source = OrtegaScans::new();
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
		let data = source
			.get_image_request(url, context)
			.unwrap()
			.data()
			.unwrap();
		assert!(data.starts_with(&[0xff, 0xd8, 0xff]));
	}

	#[aidoku_test]
	fn home_and_filters_are_populated() {
		let source = OrtegaScans::new();
		assert!(source.get_home().unwrap().components.len() >= 2);
		assert!(source.get_dynamic_filters().unwrap().len() >= 5);
	}

	#[aidoku_test]
	fn details_keep_the_series_cover() {
		let source = OrtegaScans::new();
		let manga = source
			.search(None, 1, Vec::new())
			.unwrap()
			.entries
			.into_iter()
			.next()
			.unwrap();
		let expected_cover = OrtegaScans::cover_url(&manga.key);
		let manga = source.get_manga_update(manga, true, false).unwrap();
		assert_eq!(manga.cover.as_deref(), Some(expected_cover.as_str()));

		let image = source
			.get_image_request(expected_cover, None)
			.unwrap()
			.image()
			.unwrap();
		assert!(image.width() > 0.0 && image.height() > 0.0);
	}
}
