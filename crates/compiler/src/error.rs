use ariadne::{Color, Label, Report, ReportKind, Source};





pub fn report_error(file_name: String, source: String, error_span: (usize, usize), error_type: String, message: String) {
    let span = (file_name.clone(), error_span.0..error_span.1);
    let _ = Report::build(ReportKind::Error, span.clone())
        .with_message(error_type)
        .with_label(Label::new(span)
            .with_message(message)
            .with_color(Color::Red))
        .finish()
        .print((file_name, Source::from(source)));
}