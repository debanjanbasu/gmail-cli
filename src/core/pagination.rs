use std::future::Future;

use crate::core::error::{GrrError, Result};

pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) next_page_token: Option<String>,
}

impl<T> Page<T> {
    pub(crate) fn new(items: Vec<T>, next_page_token: Option<String>) -> Self {
        Self {
            items,
            next_page_token,
        }
    }
}

pub(crate) async fn paginate<T, F, Fut>(max: Option<usize>, mut fetch: F) -> Result<Vec<T>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<Page<T>>>,
{
    let mut items = Vec::new();
    let mut page_token = None;

    loop {
        let page = fetch(page_token.clone()).await?;
        items.extend(page.items);
        if let Some(maximum) = max
            && items.len() >= maximum
        {
            items.truncate(maximum);
            break;
        }

        let Some(next_page_token) = page.next_page_token else {
            break;
        };
        if next_page_token.is_empty() {
            return Err(GrrError::Internal(
                "pagination returned an empty page token".into(),
            ));
        }
        if page_token.as_deref() == Some(next_page_token.as_str()) {
            return Err(GrrError::Internal(
                "pagination returned a repeated page token".into(),
            ));
        }
        page_token = Some(next_page_token);
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn paginates_and_truncates() -> Result<()> {
        let items = paginate(Some(3), |token| async move {
            match token.as_deref() {
                None => Ok(Page::new(vec![1, 2], Some("next".into()))),
                Some("next") => Ok(Page::new(vec![3, 4], None)),
                _ => Ok(Page::new(Vec::new(), None)),
            }
        })
        .await?;

        assert_eq!(items, vec![1, 2, 3]);
        Ok(())
    }

    #[tokio::test]
    async fn rejects_repeated_page_tokens() {
        let result = paginate(None, |_| async {
            Ok(Page::new(vec![1], Some("same".into())))
        })
        .await;

        assert!(matches!(result, Err(GrrError::Internal(message)) if message.contains("repeated")));
    }
}
