use unicode_width::UnicodeWidthStr;

use super::styled_graphemes::StyledGrapheme;

#[derive(Debug)]
pub struct Wrap<'a> {
    /// 折り返し計算をする文字列リスト
    line: &'a [StyledGrapheme],

    /// 折り返し幅
    wrap_width: Option<usize>,
}

pub trait WrapTrait {
    fn wrap(&self, wrap_width: Option<usize>) -> Wrap;
}

impl WrapTrait for Vec<StyledGrapheme> {
    fn wrap(&self, wrap_width: Option<usize>) -> Wrap {
        Wrap {
            line: &self[..],
            wrap_width,
        }
    }
}

impl<'a> Iterator for Wrap<'a> {
    type Item = &'a [StyledGrapheme];
    fn next(&mut self) -> Option<Self::Item> {
        if self.line.is_empty() {
            return None;
        }

        if let Some(wrap_width) = self.wrap_width {
            let WrapResult { wrapped, remaining } = wrap(self.line, wrap_width);

            self.line = remaining;

            Some(wrapped)
        } else {
            let ret = self.line;

            self.line = &[];

            Some(ret)
        }
    }
}

#[derive(Debug, PartialEq)]
struct WrapResult<'a> {
    wrapped: &'a [StyledGrapheme],
    remaining: &'a [StyledGrapheme],
}

fn wrap<'a>(line: &'a [StyledGrapheme], wrap_width: usize) -> WrapResult {
    let mut result = WrapResult {
        wrapped: line,
        remaining: &[],
    };

    let mut sum = 0;
    for (i, sg) in line.iter().enumerate() {
        let width = sg.symbol().width();

        if wrap_width < sum + width {
            result = WrapResult {
                wrapped: &line[..i],
                remaining: &line[i..],
            };
            break;
        }

        sum += width;
    }

    result
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::tui_wrapper::widget::text::styled_graphemes::StyledGraphemes;

    use super::*;

    #[test]
    fn 折り返しなしのときlinesを1行ずつ生成する() {
        let line = "abc".styled_graphemes();

        let actual = line.wrap(None).collect::<Vec<_>>();

        let expected = vec!["abc".styled_graphemes()];

        assert_eq!(actual, expected);
    }

    mod wrap {
        use super::*;

        use pretty_assertions::assert_eq;

        #[test]
        fn has_remaining() {
            let line: Vec<StyledGrapheme> = "0123456789".styled_graphemes();

            let result = wrap(&line, 5);

            assert_eq!(
                result,
                WrapResult {
                    wrapped: &line[..5],
                    remaining: &line[5..]
                }
            );
        }

        #[test]
        fn no_remaining() {
            let line: Vec<StyledGrapheme> = "0123456789".styled_graphemes();

            let result = wrap(&line, 10);

            assert_eq!(
                result,
                WrapResult {
                    wrapped: &line,
                    remaining: &[]
                }
            );
        }
    }

    mod 半角 {
        use super::*;

        use pretty_assertions::assert_eq;

        #[test]
        fn 折り返しのとき指定した幅に収まるリストを返す() {
            let line = "0123456789".styled_graphemes();

            let actual = line.wrap(Some(5)).collect::<Vec<_>>();

            let expected = vec![&line[..5], &line[5..]];

            assert_eq!(actual, expected);
        }
    }

    mod 全角 {
        use super::*;

        use pretty_assertions::assert_eq;

        #[test]
        fn 折り返しのとき指定した幅に収まるリストを返す() {
            let line = "アイウエオかきくけこ".styled_graphemes();

            let actual = line.wrap(Some(11)).collect::<Vec<_>>();

            let expected = vec![&line[..5], &line[5..]];

            assert_eq!(actual, expected);
        }
    }
}
