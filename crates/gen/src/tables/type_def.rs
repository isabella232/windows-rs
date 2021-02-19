use super::*;
macros::table!(TypeDef);

impl TypeDef {
    pub fn flags(&self) -> TypeFlags {
        TypeFlags(self.reader.u32(self.row, 0))
    }

    pub fn name(&self) -> &'static str {
        self.reader.str(self.row, 1)
    }

    pub fn namespace(&self) -> &'static str {
        self.reader.str(self.row, 2)
    }

    pub fn full_name(&self) -> (&'static str, &'static str) {
        (self.namespace(), self.name())
    }

    // TODO: switch uses of this to TypeDef::bases
    pub fn extends(&self) -> TypeDefOrRef {
        self.reader.decode(self.row, 3)
    }

    pub fn bases(&self) -> impl Iterator<Item = TypeDef> + '_ {
        Bases(*self)
    }

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.reader
            .list(self.row, TableIndex::Field, 4)
            .map(move |row| Field {
                reader: self.reader,
                row,
            })
    }

    pub fn methods(&self) -> impl Iterator<Item = MethodDef> + '_ {
        self.reader
            .list(self.row, TableIndex::MethodDef, 5)
            .map(move |row| MethodDef {
                reader: self.reader,
                row,
            })
    }

    pub fn generics(&self) -> impl Iterator<Item = GenericParam> + '_ {
        self.reader
            .equal_range(
                self.row.file_index,
                TableIndex::GenericParam,
                2,
                TypeOrMethodDef::TypeDef(*self).encode(),
            )
            .map(move |row| GenericParam {
                reader: self.reader,
                row,
            })
    }

    pub fn interfaces(&self) -> impl Iterator<Item = InterfaceImpl> + '_ {
        self.reader
            .equal_range(
                self.row.file_index,
                TableIndex::InterfaceImpl,
                0,
                self.row.index + 1,
            )
            .map(move |row| InterfaceImpl {
                reader: self.reader,
                row,
            })
    }

    pub fn attributes(&self) -> impl Iterator<Item = Attribute> + '_ {
        self.reader
            .equal_range(
                self.row.file_index,
                TableIndex::CustomAttribute,
                0,
                HasAttribute::TypeDef(*self).encode(),
            )
            .map(move |row| Attribute {
                reader: self.reader,
                row,
            })
    }

    pub fn has_attribute(&self, namespace: &str, name: &str) -> bool {
        self.attributes()
            .any(|attribute| attribute.full_name() == (namespace, name))
    }

    pub fn is_winrt(&self) -> bool {
        self.flags().windows_runtime()
    }

    pub fn category(&self) -> TypeCategory {
        if self.flags().interface() {
            TypeCategory::Interface
        } else {
            match self.extends().full_name() {
                ("System", "Enum") => TypeCategory::Enum,
                ("System", "MulticastDelegate") => TypeCategory::Delegate,
                ("System", "Attribute") => TypeCategory::Attribute,
                ("System", "ValueType") => {
                    if self.has_attribute("Windows.Foundation.Metadata", "ApiContractAttribute") {
                        TypeCategory::Contract
                    } else {
                        TypeCategory::Struct
                    }
                }
                _ => TypeCategory::Class,
            }
        }
    }

    pub fn guid(&self) -> Guid {
        Guid::from_type_def(self)
    }

    pub fn gen_name(&self, gen: Gen) -> TokenStream {
        let name = to_ident(self.name());
        let namespace = gen.namespace(self.namespace());
        quote! { #namespace#name }
    }

    pub fn gen_abi_name(&self, gen: Gen) -> TokenStream {
        let name = to_abi_ident(self.name());
        let namespace = gen.namespace(self.namespace());
        quote! { #namespace#name }
    }
}

struct Bases(TypeDef);

impl Iterator for Bases {
    type Item = TypeDef;

    fn next(&mut self) -> Option<Self::Item> {
        let extends = self.0.extends();

        if extends.full_name() == ("System", "Object") {
            None
        } else {
            self.0 = extends.resolve();
            Some(self.0)
        }
    }
}
