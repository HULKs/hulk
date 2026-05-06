# Tuple Struct Message Derive Design

## Goal

Add tuple struct support to the `ros_z::Message` derive macro.

## Current State

`crates/ros-z-derive/src/lib.rs` supports named structs, unit structs, and non-generic enums. It rejects tuple structs with `Message derive does not support tuple structs in v1`.

The runtime schema model represents record-like data with `TypeShape::Struct { fields }`. Each field uses `RuntimeFieldSchema::new(name, schema)`. The dynamic value layer stores struct values by field order and can also look fields up by name.

## Design

The derive macro will represent tuple structs as `TypeShape::Struct` values with positional field names.

For a tuple struct such as:

```rust
struct TupleStatus(f32, f32);
```

the generated schema will contain fields in source order:

```text
"0": f32
"1": f32
```

This keeps tuple structs on the same schema path as named structs and unit structs. It also avoids changes to `TypeShape`, dynamic CDR encoding, validation, schema hashing, and schema discovery.

Generic tuple structs will use the existing generic struct path. For example, `GenericTuple<u32>` will keep the same `Message` bounds, type-name cache, schema cache, and hash cache behavior as a named generic struct.

## Implementation Notes

`impl_message_for_struct` will stop rejecting `Fields::Unnamed`. It will map each unnamed field to a `RuntimeFieldSchema` whose name is the field index converted to a string.

The existing `generate_message_schema_tokens` helper will still validate each tuple field type. Tuple-typed fields remain unsupported because `Type::Tuple` still returns the current compile error.

## Tests

Update `crates/ros-z/tests/message_derive_ui.rs` so these cases pass:

- `tests/ui/message_derive/tuple_struct.rs`
- `tests/ui/message_derive/generic_tuple_struct.rs`

Remove the corresponding `.stderr` files after the cases move from `compile_fail` to `pass`.

Extend `crates/ros-z/tests/message_derive.rs` with runtime assertions for:

- tuple struct schema type name
- field count and field order
- numeric field names `"0"`, `"1"`
- tuple field schemas
- generic tuple struct type names and schema names

## Scope

This change adds tuple struct derive support only. It does not add tuple field support, generic enum support, lifetime generic support, const generic support, or a dedicated tuple schema shape.
