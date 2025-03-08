//! Items for working with arbitrary SemVer version strings.

use core::{
	cmp::Ordering,
	hash::Hash,
	str::Split,
};

/// Trait for objects that can be treated as SemVer version strings with parts that can be iterated on.
pub trait Version {
	/// Type for part of a version.
	/// 
	/// For SemVer, this is the major version, minor version, revision number, and so on.
	type Part<'a>: Ord where Self: 'a;

	/// Iterator over parts of a version.
	/// 
	/// See [`iter_parts`](Version::iter_parts) for more information.
	type Iter<'a>: Iterator<Item = Self::Part<'a>> where Self: 'a;

	/// Return an iterator over parts of a version.
	fn iter_parts(&self) -> Self::Iter<'_>;

	/// Return the relation of this version to `other`.
	fn relation_to(&self, other: &Self) -> Relation {
		let mut self_parts = self.iter_parts();
		let mut other_parts = other.iter_parts();
		loop {
			match (self_parts.next(), other_parts.next()) {
				(None, None) => break Relation::Equal,
				(Some(..), None) => break Relation::IsSubVersionOf,
				(None, Some(..)) => break Relation::IsSuperVersionOf,
				(Some(s), Some(o)) => {
					if s != o {
						break Relation::Different
					}
				}
			}
		}
	}

	/// Return `true` if this version is a sub-version of `other`.
	#[inline]
	fn is_sub_version_of(&self, other: &Self) -> bool {
		matches!(self.relation_to(other), Relation::Equal | Relation::IsSubVersionOf)
	}
}

/// Enumeration of kinds of relationships one version has to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relation {
	/// All version parts are equal to ones of the other version (for e.g. `1.12.0.7192` vs `1.12.0.7192`).
	Equal,
	/// Some version part is different from one of the other version (for e.g. `1.12.0.7192` vs `1.12.0.7150`).
	Different,
	/// This version is a sub-version of the other version (for e.g. `1.12.0.7192` vs `1.12`).
	IsSubVersionOf,
	/// The other version is a sub-version of this version (for e.g. `1.12` vs `1.12.0.7192`).
	IsSuperVersionOf,
}

/// Standard [`Ord`] implementation for [`Version`]s.
pub fn version_ord<V: Version + ?Sized>(a: &V, b: &V) -> Ordering {
	let mut ord = Ordering::Equal;
	for (a, b) in a.iter_parts().zip(b.iter_parts()) {
		ord = ord.then(a.cmp(&b));
	}
	ord
}

/// Helper trait for getting the length of a version part.
pub trait PartLen {
	/// Return the length of this version part.
	fn len(&self) -> usize;

	/// Return `true` if this version part is empty.
	#[inline]
	fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl PartLen for str {
	#[inline]
	fn len(&self) -> usize {
		self.len()
	}

	#[inline]
	fn is_empty(&self) -> bool {
		self.is_empty()
	}
}

impl<T> PartLen for [T] {
	#[inline]
	fn len(&self) -> usize {
		self.len()
	}

	#[inline]
	fn is_empty(&self) -> bool {
		self.is_empty()
	}
}

/// Comparison adapter for version parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Part<'a, T: ?Sized + PartLen>(pub &'a T);
impl<T: ?Sized + Ord + PartLen> Ord for Part<'_, T> {
	fn cmp(&self, other: &Self) -> Ordering {
		match self.0.len().cmp(&other.0.len()) {
			Ordering::Equal => self.0.cmp(other.0),
			ord => ord,
		}
	}
}
impl<T: ?Sized + Ord + PartLen> PartialOrd for Part<'_, T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Version for str {
	type Part<'a> = Part<'a, str> where Self: 'a;
	type Iter<'a> = VersionStrSplit<'a> where Self: 'a;
	#[inline]
	fn iter_parts(&self) -> Self::Iter<'_> {
		VersionStrSplit(self.split('.'))
	}
}

impl Version for String {
	type Part<'a> = Part<'a, str>;
	type Iter<'a> = VersionStrSplit<'a>;
	#[inline]
	fn iter_parts(&self) -> Self::Iter<'_> {
		VersionStrSplit(self.split('.'))
	}
}

/// Iterator adapter for for [`core::str::Split`] that yields [`Part`]s.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct VersionStrSplit<'a>(pub Split<'a, char>);
impl<'a> Iterator for VersionStrSplit<'a> {
	type Item = Part<'a, str>;
	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(Part)
	}
}
