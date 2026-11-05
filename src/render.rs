use crate::quota::Window;
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn countdown(seconds: i64) -> String {
    match seconds.max(0) {
        0..=59 => "now".into(),
        s if s < 3600 => format!("{} min", s / 60),
        s if s < 86400 => format!("{} hr", s / 3600),
        s => format!("{} {}", s / 86400, if s < 172800 { "day" } else { "days" }),
    }
}
pub fn status(message: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144"><rect width="144" height="144" fill="#15191f"/><text x="72" y="78" text-anchor="middle" fill="#f5c451" font-family="sans-serif" font-size="18">{}</text></svg>"##,
        escape(message)
    )
}
pub fn usage(w: &Window, name: &str, now: i64, show_percent: bool, stale: bool) -> String {
    let p = w.used_percent;
    let color = if stale {
        "#89929f"
    } else if p < 50.0 {
        "#2ecc71"
    } else if p < 85.0 {
        "#f1c40f"
    } else {
        "#e74c3c"
    };
    let circumference = std::f64::consts::TAU * 56.0;
    let offset = circumference * (1.0 - p.clamp(0.0, 100.0) / 100.0);
    let label = escape(&w.label(name));
    let remaining = w
        .resets_at
        .map(|t| countdown(t - now))
        .unwrap_or_else(|| "--".into());
    let percent = if show_percent {
        format!("{p:.0}% used")
    } else {
        String::new()
    };
    let flag = if stale { "STALE" } else { "CODEX" };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144"><rect width="144" height="144" fill="#15191f"/><circle cx="72" cy="72" r="56" fill="none" stroke="#343c48" stroke-width="10"/><circle cx="72" cy="72" r="56" fill="none" stroke="{color}" stroke-width="10" stroke-dasharray="{circumference}" stroke-dashoffset="{offset}" transform="rotate(-90 72 72)"/><g font-family="sans-serif" text-anchor="middle"><text x="72" y="44" fill="#aab2bf" font-size="10">{flag} · {label}</text><text x="72" y="80" fill="#ffffff" font-size="22">{remaining}</text><text x="72" y="103" fill="{color}" font-size="13">{percent}</text></g></svg>"##
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_and_rounding() {
        assert_eq!(countdown(-1), "now");
        assert_eq!(countdown(3600), "1 hr");
        assert_eq!(countdown(86400), "1 day");
    }
    #[test]
    fn safely_renders_stale_and_over_quota() {
        let w = Window {
            used_percent: 110.0,
            window_duration_mins: None,
            resets_at: None,
        };
        let svg = usage(&w, "<bad>", 0, true, true);
        assert!(svg.contains("STALE"));
        assert!(svg.contains("&lt;BAD&gt;"));
        assert!(svg.contains("110% used"));
        assert!(svg.contains("stroke-dashoffset=\"0\""));
    }
}
