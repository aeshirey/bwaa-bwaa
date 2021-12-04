use askama::Template;

#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchResults<'a> {
    pub results: Vec<&'a crate::song::Song>,
}
