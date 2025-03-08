use rookup_common_base::{
	toml_edit::{
		ser::to_document,
		Item, DocumentMut,
	},
	ConfigData,
};
use std::mem::take;

use crate::{
	doc_map::ItemDocMap,
	anyhow::Result as AResult,
};

#[derive(Debug)]
struct DocContext {
	pub uses_item_docs: bool,
}

/// Create a documented `config.toml` with default values.
pub fn create_default_config() -> AResult<DocumentMut> {
	let mut doc_map = ItemDocMap::new();

	let config = ConfigData::default();
	doc_map.register_with(
		&config.source,
		DocContext {
			uses_item_docs: true,
		},
	);
	doc_map.register_with(
		&config,
		DocContext {
			uses_item_docs: false,
		},
	);

	let mut config_toml = to_document(&config)?;
	{
		let mut to_doc = vec![
			(config_toml.iter_mut(), doc_map.get::<ConfigData>().unwrap())
		];
		let mut had_empty_table = false;
		while let Some((iter, table_doc)) = to_doc.pop() {
			let mut iter = iter.peekable();
			while let Some((mut key, item)) = iter.next() {
				if let Some(field) = table_doc.field_kebab_case(key.get()) {
					let field_doc = doc_map.get_by_id(field.type_id);
					let doc_string = match field_doc {
						Some(field_doc) if field_doc.extra.uses_item_docs => field_doc.item_docs,
						_ => field.docs,
					};

					let doc = to_toml_doc(doc_string, had_empty_table);
					had_empty_table = false;

					match take(item).into_table() {
						Ok(mut t) => {
							if t.is_empty() {
								had_empty_table = true;
							}

							let decor = t.decor_mut();
							decor.set_prefix(doc);

							*item = Item::Table(t);

							if let Some(field_doc) = field_doc {
								let t = item.as_table_mut().expect("`into_table` turned `value` into a `Table`");
								to_doc.push((t.iter_mut(), field_doc));
							}
						}
						Err(mut i) => {
							key.leaf_decor_mut().set_prefix(doc);
							if iter.peek().is_some() {
								if let Some(value) = i.as_value_mut() {
									value.decor_mut().set_suffix("\n");
								}
							}

							*item = i;
						}
					}
				}
			}
		}
	}

	Ok(config_toml)
}

fn to_toml_doc(doc: &str, push_nl: bool) -> String {
	const DOC_BEGIN: &str = "# ";
	const DOC_END: &str = "\n";
	const DOC_DECOR_LEN: usize = DOC_BEGIN.len() + DOC_END.len();

	let n_lines = doc.lines().count();
	let mut buffer = String::with_capacity(1 + doc.len() + n_lines * DOC_DECOR_LEN);
	if push_nl {
		buffer.push('\n');
	}

	for line in doc.lines() {
		buffer.push_str(DOC_BEGIN);
		buffer.push_str(line);
		buffer.push_str(DOC_END);
	}
	buffer
}
