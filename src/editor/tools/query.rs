use super::*;

impl EditorTools {
    pub fn crops(&self) -> Vec<CropElement> {
        self.collect_objects(ToolObject::as_crop)
    }

    pub fn active_text_id(&self) -> Option<u64> {
        self.active_text_box
    }

    pub fn active_text(&self) -> Option<&TextElement> {
        self.active_text_box.and_then(|id| self.get_text(id))
    }

    pub fn active_text_focus_content(&self) -> Option<&str> {
        self.active_text().map(|text| text.content.as_str())
    }

    pub fn get_crop(&self, id: u64) -> Option<&CropElement> {
        self.find_object_ref(id, ToolObject::as_crop)
    }

    pub fn get_text(&self, id: u64) -> Option<&TextElement> {
        self.find_object_ref(id, ToolObject::as_text)
    }

    pub fn get_text_mut(&mut self, id: u64) -> Option<&mut TextElement> {
        self.find_object_mut(id, ToolObject::as_text_mut)
    }

    pub fn object(&self, id: u64) -> Option<&ToolObject> {
        self.objects.iter().find(|object| object.id() == id)
    }
}
