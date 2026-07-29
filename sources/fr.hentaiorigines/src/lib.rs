#![no_std]
use aidoku::{
	Chapter, ContentRating, FilterValue, Listing, Manga, MangaPageResult, Page, PageContent,
	PageContext, Result, Source, Viewer,
	alloc::{Vec, vec},
	imports::net::Request,
	prelude::*,
};
use madara::{Impl, LoadMoreStrategy, Madara, Params};

const BASE_URL: &str = "https://hentai-origines.com";

struct HentaiOrigines;

impl Impl for HentaiOrigines {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			use_new_chapter_endpoint: true,
			use_load_more_request: LoadMoreStrategy::AutoDetect,
			default_viewer: Viewer::Webtoon,
			datetime_format: "dd/MM/yyyy".into(),
			datetime_locale: "fr_FR".into(),
			details_title_selector: ".post-title h1".into(),
			details_cover_selector: ".summary_image img".into(),
			details_author_selector: ".post-content_item a[href*='/manga-author/']".into(),
			details_artist_selector: ".post-content_item a[href*='/manga-artist/']".into(),
			details_description_selector: ".description-summary .summary__content".into(),
			details_tag_selector: ".post-content_item a[href*='/manga-genre/']".into(),
			details_status_selector:
				"div.post-content_item:contains(État) div.summary-content".into(),
			page_list_selector: ".reading-content img.wp-manga-chapter-img, .reading-content .page-break, .reading-content img[data-src], .reading-content img[data-lazy-src]".into(),
			..Default::default()
		}
	}

	fn get_manga_content_rating(
		&self,
		_html: &aidoku::imports::html::Document,
		_manga: &Manga,
	) -> ContentRating {
		ContentRating::NSFW
	}

	fn modify_request(&self, _params: &Params, request: Request) -> aidoku::Result<Request> {
		Ok(request.header("Cookie", "wpmanga-adault=1"))
	}

	fn get_manga_list(
		&self,
		params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let index = match listing.id.as_str() {
			"popular" => 4,
			"views" => 5,
			"new" => 6,
			_ => 1,
		};
		self.get_search_manga_list(
			params,
			None,
			page,
			vec![FilterValue::Sort {
				id: "m_orderby".into(),
				index,
				ascending: false,
			}],
		)
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = self.modify_request(params, Request::get(&url)?)?.html()?;
		let mut context = PageContext::new();
		context.insert("Referer".into(), url.clone());
		Ok(html
			.select(".reading-content img.wp-manga-chapter-img, .reading-content .page-break img, .reading-content img[data-src], .reading-content img[data-lazy-src]")
			.map(|els| {
				els.filter_map(|image| {
					let image_url = image
						.attr("data-src")
						.or_else(|| image.attr("data-lazy-src"))
						.or_else(|| image.attr("abs:src"))?;
					Some(Page {
						content: PageContent::url_context(
							madara::helpers::absolute_url(&url, &image_url),
							context.clone(),
						),
						..Default::default()
					})
				})
				.collect()
			})
			.unwrap_or_default())
	}
}

register_source!(
	Madara<HentaiOrigines>,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	PageImageProcessor,
	ListingProvider,
	MigrationHandler
);

#[cfg(test)]
mod test {
	use super::*;
	use aidoku::{DynamicFilters, Home, ImageRequestProvider};
	use aidoku_test::aidoku_test;

	#[aidoku_test]
	fn reader_returns_a_decodable_image() {
		let source = Madara::<HentaiOrigines>::new();
		let manga = source
			.get_search_manga_list(None, 1, Vec::new())
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
		let source = Madara::<HentaiOrigines>::new();
		assert!(!source.get_home().unwrap().components.is_empty());
		assert!(source.get_dynamic_filters().unwrap().len() >= 6);
	}
}
