use std::fmt::Display;

use colour::formatted::{Colour, Formatted};

use super::{
    Error,
    reportable::{Ctxt, Reportable},
};

/// What `Error`s get turned into before they can be displayed
pub struct Report {
    ctxts: Vec<Ctxt>,
    title: String,
}

impl<E> From<Error<E>> for Report
where
    E: Reportable,
{
    fn from(mut value: Error<E>) -> Self {
        let ctxts = value.ctxt();
        let title = format!(
            "{}: {}",
            Formatted::from(format!("error[E{}]", value.code()))
                .bold()
                .colour(Colour::Red),
            Formatted::from(value.msg()).bold()
        );

        Self { ctxts, title }
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.title)?;

        for ctxt in &self.ctxts {
            let path = format!(
                " {} {}",
                Formatted::from(String::from("-->"))
                    .bold()
                    .colour(Colour::Cyan),
                ctxt.inner.path
            );

            let line_num = format!(" {} ", ctxt.inner.num);
            let gutter_size = line_num.chars().count();

            let line_gutter = Formatted::from(format!("{}| ", line_num))
                .bold()
                .colour(Colour::Cyan);
            let line_padding = Formatted::from(format!("{}| ", " ".repeat(gutter_size)))
                .bold()
                .colour(Colour::Cyan);

            let mut line_pad = String::from("");
            let mut uptick = String::from("");
            let mut column = 0;

            let (start, end) = (ctxt.inner.annotation_range.0, ctxt.inner.annotation_range.1);
            for (pos, _) in ctxt.inner.line.char_indices() {
                if pos < start {
                    line_pad += " ";
                    column += 1;
                } else if start <= pos && pos < end || start == end {
                    uptick += "^";
                }
            }

            let path_line = format!("{}:{}:{}", path, ctxt.inner.num, column);

            writeln!(f, "{}", path_line)?;

            writeln!(f, "{line_padding}")?;
            writeln!(f, "{}{}", line_gutter, ctxt.inner.line)?;

            writeln!(
                f,
                "{line_padding}{line_pad}{} {}",
                Formatted::from(uptick).bold().colour(Colour::Red),
                Formatted::from(ctxt.annotation.to_string())
                    .bold()
                    .colour(Colour::Red),
            )?;

            writeln!(f, "{}", line_padding)?;
        }

        Ok(())
    }
}
