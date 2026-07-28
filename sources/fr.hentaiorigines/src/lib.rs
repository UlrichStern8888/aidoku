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
	Madara<HentaiOrigines>,
	DeepLinkHandler,
	DynamicFilters,
	Home,
	ImageRequestProvider,
	ListingProvider,
	MigrationHandler
);
