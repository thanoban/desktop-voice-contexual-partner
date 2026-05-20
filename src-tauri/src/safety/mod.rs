#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistressLevel {
    None,
    Mild,
    Severe,
}

pub fn check(input: &str) -> DistressLevel {
    let lower = input.to_lowercase();

    let severe = [
        "kill myself", "end my life", "want to die", "don't want to live",
        "suicidal", "suicide", "self harm", "self-harm", "cut myself",
        "hurt myself", "no reason to live", "can't go on",
        "don't want to be here anymore", "better off dead",
    ];

    let mild = [
        "feeling hopeless", "what's the point", "nobody cares about me",
        "so lonely", "completely alone", "worthless", "hate myself",
        "everything is pointless", "can't take it anymore",
    ];

    if severe.iter().any(|p| lower.contains(p)) {
        DistressLevel::Severe
    } else if mild.iter().any(|p| lower.contains(p)) {
        DistressLevel::Mild
    } else {
        DistressLevel::None
    }
}

pub fn companion_suffix(level: DistressLevel) -> Option<&'static str> {
    match level {
        DistressLevel::None => None,
        DistressLevel::Mild => Some(
            "\n\nI also want to gently say — what you're feeling matters, \
             and you don't have to carry it alone. Real support is always available."
        ),
        DistressLevel::Severe => Some(
            "\n\nI care about you deeply. Please reach out to a crisis line right now — \
             there are trained humans there who can help in ways I truly cannot. \
             You deserve real support."
        ),
    }
}
