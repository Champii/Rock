use crate::mir::MirClosureCaptureKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    SharedBorrow,
    MutableBorrow,
    Move,
}

impl From<MirClosureCaptureKind> for CaptureKind {
    fn from(value: MirClosureCaptureKind) -> Self {
        match value {
            MirClosureCaptureKind::ByRef => CaptureKind::SharedBorrow,
            MirClosureCaptureKind::ByMutRef => CaptureKind::MutableBorrow,
            MirClosureCaptureKind::ByValue => CaptureKind::Move,
        }
    }
}
