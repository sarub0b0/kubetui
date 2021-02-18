#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

// module.exports.formatDuration = function (duration) {
//   if (duration.years() > 0)
//     return duration.format('y[y] M[M]');
//   else if (duration.months() > 0)
//     return duration.format('M[M] d[d]');
//   else if (duration.days() > 0)
//     return duration.format('d[d] h[h]');
//   else if (duration.hours() > 0)
//     return duration.format('h[h] m[m]');
//   else if (duration.minutes() > 0)
//     return duration.format('m[m] s[s]');
//   else
//     return duration.format('s[s]');
// }
//

pub fn age(duration: &Duration) -> String {
    let duration_seconds = duration.num_seconds();

    let seconds = duration_seconds % 60;
    let minutes = (duration_seconds / 60) % 60;
    let hours = (minutes / 60) % 24;
    let days = hours / 24;

    if 0 < days {
        return format!("{}d", days);
    }
    if 0 < hours {
        return format!("{}h", hours);
    }
    if 0 < minutes {
        return format!("{}m", minutes);
    }
    return format!("{}s", seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds() {
        let duration = Duration::seconds(6);

        assert_eq!(age(&duration), "6s")
    }
    #[test]
    fn minutes() {}
    #[test]
    fn hours() {}
}
