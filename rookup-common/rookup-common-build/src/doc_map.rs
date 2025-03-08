use core::any::{
	Any, TypeId,
};
use rookup_common_base::{
	documented::*,
	field_access::*,
};
use rustc_hash::{
	FxBuildHasher, FxHashMap,
};

/// Documentation for an item's field.
#[derive(Debug)]
pub struct FieldDoc {
	pub name: &'static str,
	pub docs: &'static str,
	pub type_id: TypeId,
}

/// Documentation for an item, with associated extra data of type `D`.
#[derive(Debug)]
pub struct ItemDoc<D> {
	pub item_docs: &'static str,
	pub fields: Vec<FieldDoc>,
	pub extra: D,
}

impl<D> ItemDoc<D> {
	/// Use an example value of type `T` to scrape its documentation.
	pub fn new<T: ?Sized + WithDocs>(example: &T, extra: D) -> Self {
		macro_rules! unwrap_cuz_field_name {
			($o:expr) => {
				$o.expect("`field_names` should return fields that always exist")
			};
		}

		let fields = example.field_names().iter()
			.map(move |name| {
				let any = unwrap_cuz_field_name!(example.field_as_any(name));
				let docs = unwrap_cuz_field_name!(T::get_field_docs(name).ok());
				FieldDoc {
					name,
					docs,
					type_id: any.type_id(),
				}
			})
			.collect();

		Self {
			item_docs: T::DOCS,
			fields,
			extra,
		}
	}

	/// Look up a field by its name in `kebab-case`.
	pub fn field_kebab_case(&self, query: &str) -> Option<&FieldDoc> {
		self.fields.iter()
			.find(move |f| f.name.split('_').eq(query.split('-')))
	}
}

/// Trait for documented types that can be used to construct [`ItemDoc`].
pub trait WithDocs: Any + Documented + DocumentedFields + AnyFieldAccess {}
impl<T: Any + Documented + DocumentedFields + AnyFieldAccess> WithDocs for T {}

/// Map of types (identified by their [`TypeId`]s) to their [`ItemDoc`].
/// 
/// Each type must be [`registered`](Self::register_with) to appear in the map, because [`ItemDoc`] requires an example
/// value to get field [`TypeId`]s.
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ItemDocMap<D = ()>(pub FxHashMap<TypeId, ItemDoc<D>>);
impl<D> ItemDocMap<D> {
	/// Create a new empty map.
	#[inline]
	pub fn new() -> Self {
		Self(FxHashMap::with_hasher(FxBuildHasher))
	}

	/// Register type `T` in the map, given `extra` associated data and an `example` value.
	pub fn register_with<'a, T: ?Sized + WithDocs>(&'a mut self, example: &T, extra: D) -> &'a mut ItemDoc<D> {
		let type_id = example.type_id();
		self.0.entry(type_id)
			.or_insert_with(move || ItemDoc::new::<T>(example, extra))
	}

	/// Try to get the [`ItemDoc`] associated with `T`.
	#[inline]
	pub fn get<T: ?Sized + Any>(&self) -> Option<&ItemDoc<D>> {
		self.0.get(&TypeId::of::<T>())
	}

	/// Try to get the [`ItemDoc`] associated with `type_id`.
	#[inline]
	pub fn get_by_id(&self, type_id: TypeId) -> Option<&ItemDoc<D>> {
		self.0.get(&type_id)
	}
}

impl ItemDocMap<()> {
	// TODO: Remove this method?
	#[allow(dead_code)]
	/// Register type `T` in the map, given an `example` value and no extra associated data.
	#[inline]
	pub fn register<'a, T: ?Sized + WithDocs>(&'a mut self, example: &T) -> &'a mut ItemDoc<()> {
		self.register_with(example, ())
	}
}
