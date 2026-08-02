#![no_std]
//! Aidoku adapter for Hentai Scantrad VF, built on the shared Madara engine.

use aidoku::{
	ContentRating, FilterValue, Listing, Manga, MangaPageResult, Result, Source, Viewer,
	alloc::vec, prelude::*,
};
use madara::{Impl, Madara, Params};

const BASE_URL: &str = "https://hentai.scantrad-vf.cc";

struct HentaiScantradVf;

impl Impl for HentaiScantradVf {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			use_new_chapter_endpoint: true,
			default_viewer: Viewer::Webtoon,
			datetime_locale: "fr_FR".into(),
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
	Madara<HentaiScantradVf>,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	PageImageProcessor,
	ListingProvider,
	MigrationHandler
);
