# Const Generic Message Derive Design

## Goal

Add const generic struct support to the `ros_z::Message` derive macro.

## Current State

`crates/ros-z-derive/src/lib.rs` rejects const generic structs in `ensure_supported_struct_generics` with `Message derive does not support const generics in v1`.

The struct derive path already supports generic type parameters. It adds bounds for type parameters, builds type names from generic argument type names, and caches type names, schemas, and schema hashes by `TypeId`. The `TypeId` cache key can distinguish `Fixed<4>` from `Fixed<8>` once the derive allows const parameters.

`ros_z::Message` already supports arrays with `impl<T, const N: usize> Message for [T; N]`. Array type names include the const value, such as `[u8;4]`, and array schemas use `SequenceLength::Fixed(N)`.

## Design

The derive macro will support const generic parameters on structs. Enums will remain non-generic under the existing `ensure_non_generic_enum` rule. Lifetime parameters will remain unsupported.

Derived struct type names will include const generic values in the same generic argument list as type parameters. Type parameters will keep using `<T as ::ros_z::Message>::type_name()`. Const parameters will use `::std::format!("{}", N)`.

For this type:

```rust
struct Fixed<const N: usize> {
    values: [u8; N],
}
```

the generated type names will be:

```text
message_derive::Fixed<4>
message_derive::Fixed<8>
```

For mixed type and const parameters:

```rust
struct GenericFixed<T, const N: usize> {
    values: [T; N],
}
```

the generated type name for `GenericFixed<u32, 4>` will be:

```text
message_derive::GenericFixed<u32,4>
```

This keeps the current comma-joined generic formatting and avoids adding parameter names like `N=4`.

## Implementation Notes

`ensure_supported_struct_generics` will allow `GenericParam::Const(_)` and continue rejecting `GenericParam::Lifetime(_)`.

The type-name generation in `impl_message_for_struct` will collect both type and const generic parameters in declaration order. It will render type params through `Message::type_name()` and const params through `format!("{}", PARAM)`.

`add_message_bounds` will continue adding bounds only to type parameters. Const parameters need no `Message`, `Serialize`, `Deserialize`, `Send`, `Sync`, or `'static` bounds.

The existing schema and hash caches will stay keyed by `TypeId`. No runtime schema, dynamic value, codec, schema hashing, or discovery changes are required.

## Tests

Update `crates/ros-z/tests/message_derive_ui.rs` so `tests/ui/message_derive/const_generic.rs` moves from `compile_fail` to `pass`. Remove `tests/ui/message_derive/const_generic.stderr` after the case passes.

Extend `crates/ros-z/tests/message_derive.rs` with runtime assertions for:

- const generic struct type names, including distinct values such as `Fixed<4>` and `Fixed<8>`
- schema names matching those type names
- fixed array field schemas using `SequenceLength::Fixed(N)`
- mixed type and const generic names such as `GenericFixed<u32,4>`
- distinct schema hashes for different const values when the fixed sequence length differs

## Scope

This change adds const generic support for derived structs only. It does not add lifetime generic support, generic enum support, default const argument special handling, or a new runtime schema shape.
