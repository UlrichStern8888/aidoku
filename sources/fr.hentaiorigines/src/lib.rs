#![no_std]
//! Aidoku adapter for HentaiOrigines, built on the shared Madara engine.

use aidoku::{
	ContentRating, FilterValue, Listing, Manga, MangaPageResult, Result, Source, Viewer,
	alloc::vec, imports::net::Request, prelude::*,
};
use madara::{Impl, LoadMoreStrategy, Madara, Params};

const BASE_URL: &str = "https://hentai-origines.com";
const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1";

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
		// The site hides the catalogue and chapters until its adult gate cookie is
		// present. Browser headers avoid the intermittent empty/blocked response
		// returned to unidentified clients.
		Ok(request
			.header("Cookie", "wpmanga-adault=1")
			.header("User-Agent", USER_AGENT)
			.header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8"))
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
	use aidoku::{DynamicFilters, Home, ImageRequestProvider, PageContent, alloc::Vec};
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
