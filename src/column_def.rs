pub enum ColumnDefType {
    BuiltIn,
    User,
}

pub enum ColumnDefAlign {
    Left,
    Right,
    Center,
}

pub struct ColumnDef {
    pub def_type: ColumnDefType,
    pub name: String,
    pub value_def: String,
    pub width: Option<u8>,
    pub horizontal_align: ColumnDefAlign,
// - style
// - max width
// - min width
}

// Implementation block for ColumnDef
impl ColumnDef {
    // Constructor method for creating a new ColumnDef
    pub fn new(
        def_type: ColumnDefType,
        name: String,
        value_def: String,
        width: Option<u8>,
        horizontal_align: ColumnDefAlign,
    ) -> Self {
        Self {
            def_type,
            name,
            value_def,
            width,
            horizontal_align,
        }
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_def_new() {
        // Create a new ColumnDef object
        let column_def = ColumnDef::new(
            ColumnDefType::User,
            "Test Column".to_string(),
            "Default Value".to_string(),
            Some(10),
            ColumnDefAlign::Center,
        );

        // Assert that the fields are initialized correctly
        match column_def.def_type {
            ColumnDefType::User => {}
            _ => panic!("Expected ColumnDefType::User"),
        }
        assert_eq!(column_def.name, "Test Column");
        assert_eq!(column_def.value_def, "Default Value");
        assert_eq!(column_def.width, Some(10));
        match column_def.horizontal_align {
            ColumnDefAlign::Center => {}
            _ => panic!("Expected ColumnDefAlign::Center"),
        }
    }
}
