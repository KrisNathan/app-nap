/// Variant order is significant: derived Ord ranks later variants higher.
/// Wakefulness: Performance > BackgroundBusy > BackgroundIdle > NapBusy > NapIdle
///
/// Example:
/// ```rust
/// let cmp = Policy::Performance > Policy::BackgroundBusy;
/// assert_eq!(cmp, true);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Policy {
    NapIdle,
    NapBusy,
    BackgroundIdle,
    BackgroundBusy,
    Performance,
}
