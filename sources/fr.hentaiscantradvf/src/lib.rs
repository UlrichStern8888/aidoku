#![no_std]
use aidoku::{
	Chapter, ContentRating, FilterValue, Listing, Manga, MangaPageResult, Page, PageContent,
	PageContext, Result, Source, Viewer,
	alloc::{Vec, vec},
	imports::net::Request,
	prelude::*,
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

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?.html()?;
		let mut context = PageContext::new();
		context.insert("Referer".into(), url);
		Ok(html
			.select(".reading-content img.wp-manga-chapter-img, .reading-content .page-break img, .reading-content img[data-src], .reading-content img[data-lazy-src]")
			.map(|els| {
				els.filter_map(|image| {
					let url = image
						.attr("data-src")
						.or_else(|| image.attr("data-lazy-src"))
						.or_else(|| image.attr("abs:src"))?;
					Some(Page { content: PageContent::url_context(url.trim(), context.clone()), ..Default::default() })
				})
				.collect()
			})
			.unwrap_or_default())
	}
}

register_source!(
	Madara<HentaiScantradVf>,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	ListingProvider,
	MigrationHandler
);
