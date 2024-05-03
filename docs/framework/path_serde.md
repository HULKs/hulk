# Path Serialization and Deserialization

The `path_serde` crate introduces essential interfaces for serializing and deserializing specific parts of types by providing paths to their internals.
This functionality is mainly used by *Communication* to serialize data and provide it to connected debugging applications.

## Traits

The crate is providing three distinct traits: `PathSerialize`, `PathDeserialize`, and `PathIntrospect`.

### `PathSerialize`

The PathSerialize trait enables the serialization of specific parts of types by accepting a path to the desired internal data.
This is particularly useful when only certain portions of a data structure need to be serialized.

```rust
trait PathSerialize {
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer;
}
```

For instance a user is only interested in the position angle value of the ankle pitch joint, a serialization of the path `Control.main.sensor_data.positions.ankle_pitch` results in serializing only this specific float value.

### `PathDeserialize`

Conversely, the `PathDeserialize` trait facilitates the deserialization of data into types given a specified path.

```rust
trait PathDeserialize {
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>;
}
```

This functionality is used when changing only parts of parameters.

## `PathIntrospect`

The `PathIntrospect` trait enables type introspection, allowing the user to generate a set of available paths to fiels of a type.
This functionality is valuable for dynamically exploring the structure of data types and determining the paths that can be utilized for serialization and deserialization.
For instance, tooling may use these paths to autocomplete available paths when subscribing data from the robot.

## Macro

`path_serde` also provides derive macros, automatically generating the implementation of the three traits.
The source of an annotated type is analyzed and implementation is generated for each field, delegating the call to sub-types.

### Attributes

Types and fields can be additionally annotated with attributes to modify code generation.
Each attribute is prefixed with `#[path_serde(<...>)` to identify attributes to the `path_serde` macros.
We define the following attributes:

#### Container: `bound`

This attribute is attached to a container type and defines generic where bounds to the implementation.

```rust
#[derive(Serialize, PathSerialize)]
#[path_serde(bound = T: PathSerialize + Serialize)]
struct MyStruct<T> {
    foo: T,
}
```

#### Container: `add_leaf`

The `add_leaf` attribute adds an additional leaf to the children of a type by specifying a leaf name and a type.
This type is required to implement a `TryFrom<Self>` to generate the data for this field when requested.
Additionally, this type must be serializable or deserializable.

```rust
#[derive(Serialize, PathSerialize)]
#[path_serde(add_leaf(bar: MyIntoType)]
struct MyStruct {
    foo: i32,
}
```

#### Field: `leaf`

This attributes tags a field to be a leaf of the tree.
That means, it is not expected to have further children, and path delegation ends at this field.

```rust
#[derive(Serialize, PathSerialize)]
pub struct MultivariateNormalDistribution {
    pub mean: f32,
    #[path_serde(leaf)]
    pub covariance: FancyType,
}
```

#### Field: `skip`

This attributes tags a field to be skipped for the implementation.
It is neither considered for (de-)serialization, nor included in the available paths.

```rust
#[derive(Serialize, PathSerialize)]
pub struct MultivariateNormalDistribution {
    pub mean: f32,
    #[path_serde(skip)]
    pub covariance: FancyType,
}
```

## Example Usage

```rust
#[derive(PathSerialize, PathDeserialize, PathIntrospect)]
struct ExampleStruct {
    foo: u32,
    bar: String,
}

fn main() {
    let example = ExampleStruct {
        foo: 42,
        bar: String::from("example"),
    };

    // Serialize data using path
    let serialized_data = example.serialize_path("foo", /* serializer */);

    // Deserialize data from a specified path
    let deserialized_data = example.deserialize_path("bar", /* deserializer */);

    // Generate a set of all available paths within ExampleStruct
    let available_paths = ExampleStruct::get_fields();
}
```
