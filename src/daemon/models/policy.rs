use serde::Serialize;
use zbus::zvariant::Type;

/// Variant order is significant: derived Ord ranks later variants higher.
/// Wakefulness: Performance > BackgroundBusy > BackgroundIdle > NapBusy > NapIdle
///
/// Example:
/// ```rust
/// let cmp = Policy::Performance > Policy::BackgroundBusy;
/// assert_eq!(cmp, true);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Debug, Type)]
#[zvariant(signature = "s")]
pub enum Policy {
    NapIdle,
    NapBusy,
    BackgroundIdle,
    BackgroundBusy,
    Performance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_the_variant_name() {
        let ctxt = zbus::zvariant::serialized::Context::new_dbus(zbus::zvariant::LE, 0);
        let encoded = zbus::zvariant::to_bytes(ctxt, &Policy::Performance).unwrap();
        let decoded: String = encoded.deserialize().unwrap().0;

        assert_eq!(decoded, "Performance");
    }
}
