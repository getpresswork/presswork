use futures::channel::oneshot;
use presswork_typst::{Analysis, Tooltip};

pub(crate) fn tooltip(
    analysis: Analysis,
    text: String,
    offset: usize,
) -> impl Future<Output = Option<Tooltip>> + Send + 'static {
    run(move || analysis.tooltip(&text, offset))
}

fn run<T: Send + 'static>(
    job: impl FnOnce() -> Option<T> + Send + 'static,
) -> impl Future<Output = Option<T>> + Send + 'static {
    let (sender, receiver) = oneshot::channel();
    let spawned = std::thread::Builder::new()
        .name("typst-ide".to_owned())
        .spawn(move || {
            let _ = sender.send(job());
        });
    if let Err(error) = spawned {
        tracing::error!(%error, "The editor help thread did not start");
    }
    async move { receiver.await.ok().flatten() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Compiler;

    fn analysis(text: &str) -> Analysis {
        Compiler::new().run_now(text).unwrap().analysis.unwrap()
    }

    #[test]
    fn the_tooltip_job_gives_the_tooltip_of_the_text() {
        let text = "#let gap = 10pt";

        let tooltip = futures::executor::block_on(tooltip(
            analysis(text),
            text.to_owned(),
            text.find("10pt").unwrap() + 1,
        ));

        assert!(
            matches!(&tooltip, Some(Tooltip::Code(code)) if code.starts_with("10pt")),
            "{tooltip:?}"
        );
    }
}
