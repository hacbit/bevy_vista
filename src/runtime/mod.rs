pub mod widget_doc;

pub mod prelude {
    pub use super::widget_doc::{
        WidgetDocError, WidgetDocId, WidgetDocInstanceId, WidgetDocLiveMut, WidgetDocLiveRef,
        WidgetDocUtility,
    };
}
