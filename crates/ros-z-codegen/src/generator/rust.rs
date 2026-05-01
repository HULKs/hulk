use std::collections::{HashMap, HashSet};

use color_eyre::eyre::{Result, bail};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::types::{ArrayType, Field, FieldType, ResolvedMessage, ResolvedService, schema_fields};

/// Context for code generation, tracking external vs local packages
#[derive(Default, Clone)]
pub struct GenerationContext {
    /// External crate path for standard message types (e.g., "ros_z_msgs")
    pub external_crate: Option<String>,
    /// Set of local package names (packages being generated in this crate)
    pub local_packages: HashSet<String>,
}

impl GenerationContext {
    /// Create a new generation context
    pub fn new(external_crate: Option<String>, local_packages: HashSet<String>) -> Self {
        Self {
            external_crate,
            local_packages,
        }
    }

    /// Check if a package is local (being generated in this crate)
    pub fn is_local_package(&self, package: &str) -> bool {
        self.local_packages.is_empty() || self.local_packages.contains(package)
    }
}

fn generated_type_name(msg: &ResolvedMessage, _ctx: &GenerationContext) -> String {
    msg.schema.root.as_str().to_string()
}

// ── Plainness detection ───────────────────────────────────────────────────────

/// Returns true if the field type has a CDR wire layout identical to its
/// in-memory layout (i.e., it qualifies for `CdrPlain`).
///
/// A type is plain iff:
/// - It is a fixed-size numeric primitive (not bool, string, char/wchar).
/// - OR it is a nested struct that is itself plain (looked up in `plain_types`).
/// - AND it has no unbounded/bounded sequence dimension.
///
/// Fixed arrays of plain types are themselves plain.
pub fn is_field_plain(field_type: &FieldType, plain_types: &HashSet<String>) -> bool {
    // Sequences (Vec<T>) are never plain — they carry a variable-length prefix.
    if matches!(
        field_type.array,
        ArrayType::Unbounded | ArrayType::Bounded(_)
    ) {
        return false;
    }
    if matches!(field_type.array, ArrayType::Fixed(n) if n > 32) {
        return false;
    }
    // base type check
    is_base_type_plain(
        &field_type.base_type,
        field_type.package.as_deref(),
        plain_types,
    )
}

fn is_base_type_plain(
    base_type: &str,
    package: Option<&str>,
    plain_types: &HashSet<String>,
) -> bool {
    match base_type {
        // bool: CDR bool is u8(0|1), bytemuck::Pod not impl'd for bool
        "bool" | "string" | "wstring" | "char" | "wchar" => false,
        "byte" | "uint8" | "u8" | "int8" | "i8" | "uint16" | "u16" | "int16" | "i16" | "uint32"
        | "u32" | "int32" | "i32" | "uint64" | "u64" | "int64" | "i64" | "float32" | "f32"
        | "float64" | "f64" => true,
        custom => {
            // Look up in the set of already-confirmed-plain struct types.
            let key = match package {
                Some(pkg) => format!("{}::{}", pkg, custom),
                None => custom.to_string(),
            };
            plain_types.contains(&key)
        }
    }
}

/// Returns the alignment (in bytes) of a primitive base type, or None for
/// non-primitive / custom types.
fn primitive_align(base_type: &str) -> Option<usize> {
    match base_type {
        "byte" | "uint8" | "u8" | "int8" | "i8" => Some(1),
        "uint16" | "u16" | "int16" | "i16" => Some(2),
        "uint32" | "u32" | "int32" | "i32" | "float32" | "f32" => Some(4),
        "uint64" | "u64" | "int64" | "i64" | "float64" | "f64" => Some(8),
        _ => None,
    }
}

/// Compute the set of plain struct types for a slice of resolved messages.
///
/// Returns a `HashSet<String>` where each entry is `"package::TypeName"`.
/// The computation is bottom-up: a struct is plain iff all its fields are plain
/// AND the struct has no inter-field or trailing padding in C/Rust repr(C).
///
/// Padding detection: a struct has no padding iff all fields share the same
/// alignment (or more precisely, every field's alignment divides the struct's
/// natural alignment uniformly). We use the conservative rule: all primitive
/// fields must have the same alignment. Nested plain structs are assumed to
/// already satisfy this invariant.
pub fn compute_plain_types(messages: &[ResolvedMessage]) -> Result<HashSet<String>> {
    // Iterate until stable (handles mutually-dependent plain structs, though rare).
    let mut plain: HashSet<String> = HashSet::new();
    let mut plain_alignments: HashMap<String, usize> = HashMap::new();
    loop {
        let before = plain.len();
        'msg: for msg in messages {
            let key = format!("{}::{}", msg.parsed.package, msg.parsed.name);
            if plain.contains(&key) {
                continue;
            }
            let fields = schema_fields(msg)?;
            let mut max_align: Option<usize> = None;
            for field in &fields {
                let align = match field_alignment(field, &msg.parsed.package, &plain_alignments) {
                    Some(align) => align,
                    None => continue 'msg,
                };
                match max_align {
                    None => max_align = Some(align),
                    Some(existing) if existing != align => continue 'msg,
                    _ => {}
                }
            }
            plain.insert(key.clone());
            plain_alignments.insert(key, max_align.unwrap_or(1));
        }
        if plain.len() == before {
            break; // stable
        }
    }
    Ok(plain)
}

fn field_alignment(
    field: &Field,
    source_package: &str,
    plain_alignments: &HashMap<String, usize>,
) -> Option<usize> {
    if matches!(
        field.field_type.array,
        ArrayType::Unbounded | ArrayType::Bounded(_)
    ) {
        return None;
    }
    if matches!(field.field_type.array, ArrayType::Fixed(n) if n > 32) {
        return None;
    }

    primitive_align(&field.field_type.base_type).or_else(|| {
        let package = field
            .field_type
            .package
            .as_deref()
            .unwrap_or(source_package);
        plain_alignments
            .get(&format!("{}::{}", package, field.field_type.base_type))
            .copied()
    })
}

// ── CDR trait codegen ─────────────────────────────────────────────────────────

/// Generate `CdrEncode`, `CdrDecode`, `CdrEncodedSize` impls,
/// and (when the struct is plain) the `CdrPlain` + `bytemuck::Pod/Zeroable` derives.
fn generate_cdr_impls(
    msg: &ResolvedMessage,
    plain_types: &HashSet<String>,
    ctx: &GenerationContext,
) -> Result<TokenStream> {
    let name = format_ident!("{}", msg.parsed.name);
    let fields = schema_fields(msg)?;
    let pkg = &msg.parsed.package;
    let is_plain = plain_types.contains(&format!("{}::{}", pkg, &msg.parsed.name));

    // ── CdrEncode ──────────────────────────────────────────────────────────
    let ser_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| generate_cdr_encode_field(f, pkg, plain_types, ctx))
        .collect::<Result<Vec<_>>>()?;

    let ser_impl = quote! {
        impl ::ros_z_cdr::CdrEncode for #name {
            fn cdr_encode<BO, B>(
                &self,
                __w: &mut ::ros_z_cdr::CdrWriter<'_, BO, B>,
            )
            where
                BO: ::byteorder::ByteOrder,
                B: ::ros_z_cdr::CdrBuffer,
            {
                #(#ser_fields)*
            }
        }
    };

    // ── CdrDecode ────────────────────────────────────────────────────────
    let de_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| generate_cdr_decode_field(f, pkg, plain_types, ctx))
        .collect();

    let field_idents: Vec<Ident> = fields.iter().map(|f| escape_field_name(&f.name)).collect();

    let de_impl = quote! {
        impl ::ros_z_cdr::CdrDecode for #name {
            fn cdr_decode<'__de, BO>(
                __r: &mut ::ros_z_cdr::CdrReader<'__de, BO>,
            ) -> ::ros_z_cdr::Result<Self>
            where
                BO: ::byteorder::ByteOrder,
            {
                #(#de_fields)*
                Ok(Self { #(#field_idents),* })
            }
        }
    };

    // ── CdrEncodedSize ─────────────────────────────────────────────────────
    let size_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| generate_cdr_size_field(f, pkg, plain_types, ctx))
        .collect::<Result<Vec<_>>>()?;

    let size_impl = quote! {
        impl ::ros_z_cdr::CdrEncodedSize for #name {
            fn cdr_encoded_size(&self, __pos: usize) -> usize {
                let mut __p = __pos;
                #(#size_fields)*
                __p
            }
        }
    };

    // ── CdrPlain (only when struct is plain) ──────────────────────────────────
    let plain_impl = if is_plain {
        quote! {
            #[cfg(target_endian = "little")]
            unsafe impl ::ros_z_cdr::CdrPlain for #name {}
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #ser_impl
        #de_impl
        #size_impl
        #plain_impl
    })
}

/// Generate a single field's CdrEncode statement.
fn generate_cdr_encode_field(
    field: &Field,
    source_pkg: &str,
    plain_types: &HashSet<String>,
    _ctx: &GenerationContext,
) -> Result<TokenStream> {
    let fname = escape_field_name(&field.name);
    let ft = &field.field_type;

    // ZBuf fields: byte sequences stored as ros_z::ZBuf (zero-copy Zenoh type).
    // CdrEncode is implemented for ros_z::ZBuf in the ros-z crate.
    if is_zbuf_field(field) {
        return Ok(quote! {
            ::ros_z_cdr::CdrEncode::cdr_encode(&self.#fname, __w);
        });
    }

    match &ft.array {
        ArrayType::Single => Ok(quote! {
            ::ros_z_cdr::CdrEncode::cdr_encode(&self.#fname, __w);
        }),
        ArrayType::Fixed(_) => {
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                Ok(quote! {
                    #[cfg(target_endian = "little")]
                    __w.write_pod_slice(&self.#fname);
                    #[cfg(not(target_endian = "little"))]
                    for __item in &self.#fname {
                        ::ros_z_cdr::CdrEncode::cdr_encode(__item, __w);
                    }
                })
            } else {
                Ok(quote! {
                    for __item in &self.#fname {
                        ::ros_z_cdr::CdrEncode::cdr_encode(__item, __w);
                    }
                })
            }
        }
        ArrayType::Unbounded | ArrayType::Bounded(_) => {
            // Check if element type is plain → bulk path
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            let bound_check = match &ft.array {
                ArrayType::Bounded(bound) => {
                    let bound = proc_macro2::Literal::usize_unsuffixed(*bound);
                    quote! {
                        assert!(
                            self.#fname.len() <= #bound,
                            "bounded sequence field `{}` exceeds max length {}",
                            stringify!(#fname),
                            #bound
                        );
                    }
                }
                _ => quote! {},
            };
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                Ok(quote! {
                    #bound_check
                    __w.write_sequence_length(self.#fname.len());
                    #[cfg(target_endian = "little")]
                    if !self.#fname.is_empty() {
                        __w.write_pod_slice(&self.#fname);
                    }
                    #[cfg(not(target_endian = "little"))]
                    for __item in &self.#fname {
                        ::ros_z_cdr::CdrEncode::cdr_encode(__item, __w);
                    }
                })
            } else {
                Ok(quote! {
                    #bound_check
                    __w.write_sequence_length(self.#fname.len());
                    for __item in &self.#fname {
                        ::ros_z_cdr::CdrEncode::cdr_encode(__item, __w);
                    }
                })
            }
        }
    }
}

/// Generate a single field's CdrDecode statement (binds a local variable).
fn generate_cdr_decode_field(
    field: &Field,
    source_pkg: &str,
    plain_types: &HashSet<String>,
    ctx: &GenerationContext,
) -> TokenStream {
    let fname = escape_field_name(&field.name);
    let ft = &field.field_type;
    let rust_elem_ty = generate_field_type_tokens_with_context(ft, source_pkg, ctx);

    // ZBuf: CdrDecode is implemented for ros_z::ZBuf in the ros-z crate.
    if is_zbuf_field(field) {
        return quote! {
            let #fname: #rust_elem_ty = ::ros_z_cdr::CdrDecode::cdr_decode(__r)?;
        };
    }

    match &ft.array {
        ArrayType::Single => quote! {
            let #fname = ::ros_z_cdr::CdrDecode::cdr_decode(__r)?;
        },
        ArrayType::Fixed(n) => {
            let n_lit = proc_macro2::Literal::usize_unsuffixed(*n);
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            let base_ty = generate_base_type_tokens_with_context(ft, source_pkg, ctx);
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                quote! {
                    let #fname: #rust_elem_ty = {
                        #[cfg(target_endian = "little")]
                        {
                            let __slice = __r.read_pod_slice::<#base_ty>(#n_lit)?;
                            ::std::convert::TryInto::try_into(__slice)
                                .map_err(|_| ::ros_z_cdr::Error::UnexpectedEof)?
                        }
                        #[cfg(not(target_endian = "little"))]
                        {
                            let mut __arr = [Default::default(); #n_lit];
                            for __slot in __arr.iter_mut() {
                                *__slot = ::ros_z_cdr::CdrDecode::cdr_decode(__r)?;
                            }
                            __arr
                        }
                    };
                }
            } else {
                quote! {
                    let #fname: #rust_elem_ty = {
                        let mut __arr = [Default::default(); #n_lit];
                        for __slot in __arr.iter_mut() {
                            *__slot = ::ros_z_cdr::CdrDecode::cdr_decode(__r)?;
                        }
                        __arr
                    };
                }
            }
        }
        ArrayType::Unbounded | ArrayType::Bounded(_) => {
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            let base_ty = generate_base_type_tokens_with_context(ft, source_pkg, ctx);
            let bound_check = match &ft.array {
                ArrayType::Bounded(bound) => {
                    let bound = proc_macro2::Literal::usize_unsuffixed(*bound);
                    quote! {
                        if __count > #bound {
                            return Err(::ros_z_cdr::Error::Custom(format!(
                                "bounded sequence field `{}` exceeds max length {}: {}",
                                stringify!(#fname),
                                #bound,
                                __count
                            )));
                        }
                    }
                }
                _ => quote! {},
            };
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                quote! {
                    let #fname: Vec<#base_ty> = {
                        let __count = __r.read_sequence_length()?;
                        #bound_check
                        #[cfg(target_endian = "little")]
                        {
                            if __count > 0 {
                                __r.read_pod_slice::<#base_ty>(__count)?
                            } else {
                                vec![]
                            }
                        }
                        #[cfg(not(target_endian = "little"))]
                        {
                            let mut __v = Vec::with_capacity(__count);
                            for _ in 0..__count {
                                __v.push(::ros_z_cdr::CdrDecode::cdr_decode(__r)?);
                            }
                            __v
                        }
                    };
                }
            } else {
                quote! {
                    let #fname: Vec<#base_ty> = {
                        let __count = __r.read_sequence_length()?;
                        #bound_check
                        let mut __v = Vec::with_capacity(__count);
                        for _ in 0..__count {
                            __v.push(::ros_z_cdr::CdrDecode::cdr_decode(__r)?);
                        }
                        __v
                    };
                }
            }
        }
    }
}

/// Generate a single field's CdrEncodedSize statement (updates `__p`).
fn generate_cdr_size_field(
    field: &Field,
    source_pkg: &str,
    plain_types: &HashSet<String>,
    _ctx: &GenerationContext,
) -> Result<TokenStream> {
    let fname = escape_field_name(&field.name);
    let ft = &field.field_type;

    // ZBuf: u32 length prefix (4-byte aligned) + byte contents.
    if is_zbuf_field(field) {
        return Ok(quote! {
            {
                use ::zenoh_buffers::buffer::Buffer;
                __p += (__p % 4 != 0) as usize * (4 - __p % 4) + 4;
                __p += self.#fname.len();
            }
        });
    }

    match &ft.array {
        ArrayType::Single => Ok(quote! {
            __p = ::ros_z_cdr::CdrEncodedSize::cdr_encoded_size(&self.#fname, __p);
        }),
        ArrayType::Fixed(_) => {
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                // O(1) size for plain fixed arrays: align + N * sizeof(T)
                Ok(quote! {
                    if !self.#fname.is_empty() {
                        let __elem_align = ::std::mem::align_of_val(&self.#fname[0]);
                        __p += (__p % __elem_align != 0) as usize
                            * (__elem_align - __p % __elem_align);
                        __p += self.#fname.len()
                            * ::std::mem::size_of_val(&self.#fname[0]);
                    }
                })
            } else {
                Ok(quote! {
                    for __item in &self.#fname {
                        __p = ::ros_z_cdr::CdrEncodedSize::cdr_encoded_size(__item, __p);
                    }
                })
            }
        }
        ArrayType::Unbounded | ArrayType::Bounded(_) => {
            let elem_plain = is_base_type_plain(
                &ft.base_type,
                ft.package.as_deref().or(Some(source_pkg)),
                plain_types,
            );
            if elem_plain && !matches!(ft.base_type.as_str(), "bool" | "string" | "wstring") {
                // O(1) size for plain sequences: align + count * sizeof(T)
                Ok(quote! {
                    // u32 sequence length prefix
                    __p += (__p % 4 != 0) as usize * (4 - __p % 4) + 4;
                    if !self.#fname.is_empty() {
                        let __elem_align = ::std::mem::align_of_val(&self.#fname[0]);
                        __p += (__p % __elem_align != 0) as usize
                            * (__elem_align - __p % __elem_align);
                        __p += self.#fname.len()
                            * ::std::mem::size_of_val(&self.#fname[0]);
                    }
                })
            } else {
                Ok(quote! {
                    // u32 sequence length prefix
                    __p += (__p % 4 != 0) as usize * (4 - __p % 4) + 4;
                    for __item in &self.#fname {
                        __p = ::ros_z_cdr::CdrEncodedSize::cdr_encoded_size(__item, __p);
                    }
                })
            }
        }
    }
}

/// Generate Rust module for a package containing messages
pub fn generate_package_module(package: &str, messages: &[ResolvedMessage]) -> Result<TokenStream> {
    let package_ident = format_ident!("{}", package);
    let message_impls: Vec<TokenStream> = messages
        .iter()
        .map(generate_message_impl)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        pub mod #package_ident {
            #(#message_impls)*
        }
    })
}

/// Generate Rust implementation for a single message
pub fn generate_message_impl(msg: &ResolvedMessage) -> Result<TokenStream> {
    generate_message_impl_with_context(msg, &GenerationContext::default())
}

/// Generate Rust implementation for a single message with external type support
pub fn generate_message_impl_with_context(
    msg: &ResolvedMessage,
    ctx: &GenerationContext,
) -> Result<TokenStream> {
    generate_message_impl_with_cdr(msg, ctx, &HashSet::new())
}

/// Generate Rust implementation with CDR trait impls and plainness information.
pub fn generate_message_impl_with_cdr(
    msg: &ResolvedMessage,
    ctx: &GenerationContext,
    plain_types: &HashSet<String>,
) -> Result<TokenStream> {
    generate_message_impl_with_cdr_options(msg, ctx, plain_types, true)
}

/// Generate Rust implementation with optional Message trait impls.
pub fn generate_message_impl_with_cdr_options(
    msg: &ResolvedMessage,
    ctx: &GenerationContext,
    plain_types: &HashSet<String>,
    generate_message_impls: bool,
) -> Result<TokenStream> {
    let name = format_ident!("{}", msg.parsed.name);
    let fields = schema_fields(msg)?;

    let msg_is_plain =
        plain_types.contains(&format!("{}::{}", msg.parsed.package, msg.parsed.name));
    let struct_def = generate_struct_with_context(
        &msg.parsed.package,
        &msg.parsed.name,
        &fields,
        &msg.parsed.constants,
        ctx,
        msg_is_plain,
    )?;
    let type_info = if generate_message_impls {
        generate_message_trait_impl(msg, ctx)?
    } else {
        quote! {}
    };

    let size_estimation_impl =
        generate_size_estimation_impl(&name, &fields, &msg.parsed.package, ctx)?;

    let cdr_impls = generate_cdr_impls(msg, plain_types, ctx)?;

    Ok(quote! {
        #struct_def
        #type_info
        #size_estimation_impl
        #cdr_impls
    })
}

/// Generate struct definition with constants
#[allow(dead_code)]
fn generate_struct(
    package: &str,
    name: &str,
    fields: &[Field],
    constants: &[crate::types::Constant],
) -> Result<TokenStream> {
    generate_struct_with_context(
        package,
        name,
        fields,
        constants,
        &GenerationContext::default(),
        false,
    )
}

/// Generate struct definition with constants (with external type support)
fn generate_struct_with_context(
    package: &str,
    name: &str,
    fields: &[Field],
    constants: &[crate::types::Constant],
    ctx: &GenerationContext,
    is_plain: bool,
) -> Result<TokenStream> {
    let name_ident = format_ident!("{}", name);
    let field_defs: Vec<TokenStream> = fields
        .iter()
        .map(|f| generate_field_def_with_context(f, package, ctx))
        .collect::<Result<Vec<_>>>()?;

    // Check if we need SmartDefault for explicit defaults or large arrays.
    let needs_smart_default = fields.iter().any(|field| {
        field.default.is_some() || matches!(&field.field_type.array, ArrayType::Fixed(n) if *n > 32)
    });

    // Generate constants as associated constants
    let const_defs: Vec<TokenStream> = constants
        .iter()
        .map(|c| {
            let const_name = format_ident!("{}", c.name);
            let const_type = generate_constant_type(&c.const_type);
            let const_value = generate_constant_value_from_string(&c.const_type, &c.value)?;
            Ok(quote! {
                pub const #const_name: #const_type = #const_value;
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let bytemuck_derives = if is_plain {
        quote! {
            #[cfg_attr(target_endian = "little", repr(C))]
            #[cfg_attr(target_endian = "little", derive(Copy, ::bytemuck::Pod, ::bytemuck::Zeroable))]
        }
    } else {
        quote! {}
    };

    if needs_smart_default {
        Ok(quote! {
            #[derive(Debug, Clone, ::smart_default::SmartDefault, ::serde::Serialize, ::serde::Deserialize)]
            #bytemuck_derives
            pub struct #name_ident {
                #(#field_defs),*
            }

            impl #name_ident {
                #(#const_defs)*
            }
        })
    } else {
        // Simple messages with standard Default
        Ok(quote! {
            #[derive(Debug, Clone, Default, ::serde::Serialize, ::serde::Deserialize)]
            #bytemuck_derives
            pub struct #name_ident {
                #(#field_defs),*
            }

            impl #name_ident {
                #(#const_defs)*
            }
        })
    }
}

/// Generate constant type tokens
fn generate_constant_type(const_type: &str) -> TokenStream {
    match const_type {
        "bool" => quote! { bool },
        "byte" | "uint8" | "char" | "u8" => quote! { u8 },
        "int8" | "i8" => quote! { i8 },
        "uint16" | "u16" => quote! { u16 },
        "int16" | "i16" => quote! { i16 },
        "uint32" | "u32" => quote! { u32 },
        "int32" | "i32" => quote! { i32 },
        "uint64" | "u64" => quote! { u64 },
        "int64" | "i64" => quote! { i64 },
        "float32" | "f32" => quote! { f32 },
        "float64" | "f64" => quote! { f64 },
        "string" => quote! { &'static str },
        _ => quote! { &'static str },
    }
}

/// Generate constant value tokens from string representation
fn generate_constant_value_from_string(const_type: &str, value: &str) -> Result<TokenStream> {
    Ok(match const_type {
        "bool" => match value {
            "true" => quote! { true },
            "false" => quote! { false },
            _ => bail!("invalid bool constant `{value}`"),
        },
        "byte" | "uint8" | "char" | "u8" | "int8" | "i8" | "uint16" | "u16" | "int16" | "i16"
        | "uint32" | "u32" | "int32" | "i32" | "uint64" | "u64" | "int64" | "i64" => {
            let expr = parse_numeric_constant_expr(value, false)?;
            quote! { #expr }
        }
        "float32" | "f32" | "float64" | "f64" => {
            let expr = parse_numeric_constant_expr(value, true)?;
            quote! { #expr }
        }
        _ => quote! { #value },
    })
}

fn parse_numeric_constant_expr(value: &str, allow_float: bool) -> Result<syn::Expr> {
    let expr = syn::parse_str::<syn::Expr>(value)
        .map_err(|error| color_eyre::eyre::eyre!("invalid numeric constant `{value}`: {error}"))?;

    let valid = match &expr {
        syn::Expr::Lit(expr_lit) => matches!(
            &expr_lit.lit,
            syn::Lit::Int(_) | syn::Lit::Float(_) if allow_float || matches!(&expr_lit.lit, syn::Lit::Int(_))
        ),
        syn::Expr::Unary(unary)
            if matches!(unary.op, syn::UnOp::Neg(_))
                && matches!(*unary.expr.clone(), syn::Expr::Lit(_)) =>
        {
            match unary.expr.as_ref() {
                syn::Expr::Lit(expr_lit) => {
                    matches!(&expr_lit.lit, syn::Lit::Int(_) | syn::Lit::Float(_) if allow_float || matches!(&expr_lit.lit, syn::Lit::Int(_)))
                }
                _ => false,
            }
        }
        _ => false,
    };

    if valid {
        Ok(expr)
    } else {
        bail!("invalid numeric constant `{value}`")
    }
}

/// Generate a field definition
#[allow(dead_code)]
fn generate_field_def(field: &Field, source_package: &str) -> Result<TokenStream> {
    generate_field_def_with_context(field, source_package, &GenerationContext::default())
}

/// Generate a field definition (with external type support)
fn generate_field_def_with_context(
    field: &Field,
    source_package: &str,
    ctx: &GenerationContext,
) -> Result<TokenStream> {
    // Escape Rust keywords with r# prefix
    let name = escape_field_name(&field.name);
    let field_type =
        generate_field_type_tokens_with_context(&field.field_type, source_package, ctx);

    let default_attribute = generate_default_attribute(field)?;

    // Add attributes for large fixed arrays (>32 elements)
    let serde_attributes = if let ArrayType::Fixed(n) = &field.field_type.array {
        if *n > 32 {
            // Generate a default value code string for the array
            let default_code = generate_array_default_code(field, *n)?;
            quote! {
                #[serde(with = "serde_big_array::BigArray")]
                #default_attribute
                #[default(_code = #default_code)]
            }
        } else {
            quote! { #default_attribute }
        }
    } else {
        quote! { #default_attribute }
    };

    Ok(quote! {
        #serde_attributes
        pub #name: #field_type
    })
}

/// Generate default value code string for an array
fn generate_array_default_code(field: &Field, size: usize) -> Result<String> {
    if let Some(default) = &field.default {
        return match default {
            crate::types::DefaultValue::BoolArray(values) => Ok(format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            crate::types::DefaultValue::IntArray(values) => Ok(format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            crate::types::DefaultValue::UIntArray(values) => Ok(format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            crate::types::DefaultValue::FloatArray(values) => Ok(format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            crate::types::DefaultValue::StringArray(values) => Ok(format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| format!("\"{}\".to_string()", value.escape_default()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            _ => bail!("invalid explicit array default for field {}", field.name),
        };
    }

    let field_type = &field.field_type;
    let elem_default = match field_type.base_type.as_str() {
        "bool" => "false",
        "byte" | "uint8" | "char" | "u8" | "int8" | "i8" | "uint16" | "u16" | "int16" | "i16"
        | "uint32" | "u32" | "int32" | "i32" | "uint64" | "u64" | "int64" | "i64" => "0",
        "float32" | "f32" | "float64" | "f64" => "0.0",
        _ => bail!(
            "Cannot generate default for large array of type {}",
            field_type.base_type
        ),
    };

    Ok(format!("[{}; {}]", elem_default, size))
}

fn generate_default_attribute(field: &Field) -> Result<TokenStream> {
    let Some(default) = &field.default else {
        return Ok(quote! {});
    };

    Ok(match default {
        crate::types::DefaultValue::Bool(value) => {
            let code = value.to_string();
            quote! { #[default(_code = #code)] }
        }
        crate::types::DefaultValue::Int(value) => {
            let code = match field.field_type.base_type.as_str() {
                "int8" | "i8" => format!("{value}i8"),
                "int16" | "i16" => format!("{value}i16"),
                "int32" | "i32" => format!("{value}i32"),
                "int64" | "i64" => format!("{value}i64"),
                "float32" | "f32" => format!("{value}f32"),
                "float64" | "f64" => format!("{value}f64"),
                _ => value.to_string(),
            };
            quote! { #[default(_code = #code)] }
        }
        crate::types::DefaultValue::UInt(value) => {
            let code = match field.field_type.base_type.as_str() {
                "byte" | "char" | "uint8" | "u8" => format!("{value}u8"),
                "uint16" | "u16" => format!("{value}u16"),
                "uint32" | "u32" => format!("{value}u32"),
                "uint64" | "u64" => format!("{value}u64"),
                _ => value.to_string(),
            };
            quote! { #[default(_code = #code)] }
        }
        crate::types::DefaultValue::Float(value) => {
            let code = match field.field_type.base_type.as_str() {
                "float32" | "f32" => format!("{value}f32"),
                _ => format!("{value}f64"),
            };
            quote! { #[default(_code = #code)] }
        }
        crate::types::DefaultValue::String(value) => {
            let code = format!("\"{}\".to_string()", value.escape_default());
            quote! { #[default(_code = #code)] }
        }
        crate::types::DefaultValue::BoolArray(_)
        | crate::types::DefaultValue::IntArray(_)
        | crate::types::DefaultValue::UIntArray(_)
        | crate::types::DefaultValue::FloatArray(_)
        | crate::types::DefaultValue::StringArray(_) => quote! {},
    })
}

/// Check if a name is a Rust keyword
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

/// Escape a field name if it's a Rust keyword
fn escape_field_name(name: &str) -> Ident {
    if is_rust_keyword(name) {
        format_ident!("r#{}", name)
    } else {
        format_ident!("{}", name)
    }
}

/// Generate Rust type tokens for a field type
#[allow(dead_code)]
fn generate_field_type_tokens(field_type: &FieldType, source_package: &str) -> TokenStream {
    generate_field_type_tokens_with_context(
        field_type,
        source_package,
        &GenerationContext::default(),
    )
}

/// Generate Rust type tokens for a field type (with external type support)
fn generate_field_type_tokens_with_context(
    field_type: &FieldType,
    source_package: &str,
    ctx: &GenerationContext,
) -> TokenStream {
    let base = generate_base_type_tokens_with_context(field_type, source_package, ctx);

    match &field_type.array {
        ArrayType::Single => base,
        ArrayType::Fixed(n) => {
            let n_lit = proc_macro2::Literal::usize_unsuffixed(*n);
            quote! { [#base; #n_lit] }
        }
        ArrayType::Unbounded => {
            // Use ros_z::ZBuf wrapper for uint8[]/byte[] (zero-copy with optimized serde)
            if matches!(field_type.base_type.as_str(), "uint8" | "byte" | "u8") {
                quote! { ::ros_z::ZBuf }
            } else {
                quote! { ::std::vec::Vec<#base> }
            }
        }
        ArrayType::Bounded(_n) => {
            // Using Vec<T> for bounded arrays (standard Rust approach)
            // Custom bounded type could be added for strict memory guarantees if needed
            quote! { ::std::vec::Vec<#base> }
        }
    }
}

/// Generate base type tokens
#[allow(dead_code)]
fn generate_base_type_tokens(field_type: &FieldType, source_package: &str) -> TokenStream {
    generate_base_type_tokens_with_context(
        field_type,
        source_package,
        &GenerationContext::default(),
    )
}

/// Generate base type tokens (with external type support)
fn generate_base_type_tokens_with_context(
    field_type: &FieldType,
    source_package: &str,
    ctx: &GenerationContext,
) -> TokenStream {
    match field_type.base_type.as_str() {
        "bool" => quote! { bool },
        "byte" | "uint8" | "char" | "u8" => quote! { u8 },
        "int8" | "i8" => quote! { i8 },
        "uint16" | "u16" => quote! { u16 },
        "int16" | "i16" => quote! { i16 },
        "uint32" | "u32" => quote! { u32 },
        "int32" | "i32" => quote! { i32 },
        "uint64" | "u64" => quote! { u64 },
        "int64" | "i64" => quote! { i64 },
        "float32" | "f32" => quote! { f32 },
        "float64" | "f64" => quote! { f64 },
        "string" => quote! { ::std::string::String },
        custom => {
            // Use explicit package or infer same package
            let pkg = field_type.package.as_deref().unwrap_or(source_package);
            let pkg_ident = format_ident!("{}", pkg);
            let type_ident = format_ident!("{}", custom);

            // Check if this is an external package reference
            if let Some(ref ext_crate) = ctx.external_crate
                && !ctx.is_local_package(pkg)
            {
                // External package - use fully qualified path
                let crate_ident = format_ident!("{}", ext_crate);
                return quote! { ::#crate_ident::#pkg_ident::#type_ident };
            }

            // Local package - use super:: as before
            quote! { super::#pkg_ident::#type_ident }
        }
    }
}

/// Generate Message trait implementation
fn generate_message_trait_impl(
    msg: &ResolvedMessage,
    ctx: &GenerationContext,
) -> Result<TokenStream> {
    let name_ident = format_ident!("{}", msg.parsed.name);
    let type_name = generated_type_name(msg, ctx);
    let hash_str = msg.schema_hash.to_hash_string();
    let fields = schema_fields(msg)?;
    let schema_field_tokens: Vec<TokenStream> = fields
        .iter()
        .map(|field| generate_schema_builder_field_tokens(field, &msg.parsed.package, ctx))
        .collect();

    Ok(quote! {
        impl ::ros_z::Message for #name_ident {
            type Codec = ::ros_z::GeneratedCdrCodec<Self>;

            fn type_name() -> &'static str {
                #type_name
            }

            fn schema_hash() -> ::ros_z::entity::SchemaHash {
                ::ros_z::entity::SchemaHash::from_hash_string(#hash_str)
                    .expect("invalid hash")
            }

            fn schema() -> ::std::sync::Arc<::ros_z::dynamic::MessageSchema> {
                static SCHEMA: ::std::sync::OnceLock<::std::sync::Arc<::ros_z::dynamic::MessageSchema>> =
                    ::std::sync::OnceLock::new();

                SCHEMA
                    .get_or_init(|| {
                        ::ros_z::dynamic::MessageSchema::builder(#type_name)
                            #(#schema_field_tokens)*
                            .schema_hash(<Self as ::ros_z::Message>::schema_hash())
                            .build()
                            .expect("generated message schema must be valid")
                    })
                    .clone()
            }
        }
    })
}

/// Generate schema builder tokens for one message field.
fn generate_schema_builder_field_tokens(
    field: &Field,
    source_package: &str,
    ctx: &GenerationContext,
) -> TokenStream {
    let field_type = generate_schema_field_type_tokens(&field.field_type, source_package, ctx);
    let field_name = &field.name;
    quote! { .field(#field_name, #field_type) }
}

/// Generate runtime dynamic `FieldType` tokens for one field.
fn generate_schema_field_type_tokens(
    field_type: &FieldType,
    source_package: &str,
    ctx: &GenerationContext,
) -> TokenStream {
    let base = generate_schema_base_field_type_tokens(field_type, source_package, ctx);

    match &field_type.array {
        ArrayType::Single => base,
        ArrayType::Fixed(n) => {
            quote! { ::ros_z::dynamic::FieldType::Array(::std::boxed::Box::new(#base), #n) }
        }
        ArrayType::Unbounded => {
            quote! { ::ros_z::dynamic::FieldType::Sequence(::std::boxed::Box::new(#base)) }
        }
        ArrayType::Bounded(n) => {
            quote! {
                ::ros_z::dynamic::FieldType::BoundedSequence(::std::boxed::Box::new(#base), #n)
            }
        }
    }
}

/// Generate runtime dynamic `FieldType` tokens for the non-array base type.
fn generate_schema_base_field_type_tokens(
    field_type: &FieldType,
    source_package: &str,
    ctx: &GenerationContext,
) -> TokenStream {
    match field_type.base_type.as_str() {
        "bool" => quote! { ::ros_z::dynamic::FieldType::Bool },
        "byte" | "uint8" | "char" | "u8" => quote! { ::ros_z::dynamic::FieldType::Uint8 },
        "int8" | "i8" => quote! { ::ros_z::dynamic::FieldType::Int8 },
        "uint16" | "u16" | "wchar" => quote! { ::ros_z::dynamic::FieldType::Uint16 },
        "int16" | "i16" => quote! { ::ros_z::dynamic::FieldType::Int16 },
        "uint32" | "u32" => quote! { ::ros_z::dynamic::FieldType::Uint32 },
        "int32" | "i32" => quote! { ::ros_z::dynamic::FieldType::Int32 },
        "uint64" | "u64" => quote! { ::ros_z::dynamic::FieldType::Uint64 },
        "int64" | "i64" => quote! { ::ros_z::dynamic::FieldType::Int64 },
        "float32" | "f32" => quote! { ::ros_z::dynamic::FieldType::Float32 },
        "float64" | "f64" => quote! { ::ros_z::dynamic::FieldType::Float64 },
        "string" => {
            if let Some(bound) = field_type.string_bound {
                quote! { ::ros_z::dynamic::FieldType::BoundedString(#bound) }
            } else {
                quote! { ::ros_z::dynamic::FieldType::String }
            }
        }
        _ => {
            let nested_type =
                generate_base_type_tokens_with_context(field_type, source_package, ctx);
            quote! {
                <#nested_type as ::ros_z::FieldTypeInfo>::field_type()
            }
        }
    }
}

/// Generate accurate size estimation implementation for SHM serialization
fn generate_size_estimation_impl(
    name: &Ident,
    fields: &[Field],
    source_package: &str,
    ctx: &GenerationContext,
) -> Result<TokenStream> {
    // Always generate size estimation for all messages (even fixed-size ones)
    // This ensures nested messages can call estimated_cdr_size() on their fields

    // Generate size calculation for each field
    let field_size_exprs: Vec<TokenStream> = fields
        .iter()
        .map(|f| generate_field_size_expr(f, source_package, ctx))
        .collect::<Result<Vec<_>>>()?;

    // Only mark size as mutable if we have fields to add
    let size_decl = if fields.is_empty() {
        quote! { let size = 0usize; }
    } else {
        quote! { let mut size = 0usize; }
    };
    let padding_slack = if fields.is_empty() {
        quote! {}
    } else {
        // Worst-case CDR alignment padding before each field is 7 bytes.
        quote! { size += 7usize; }
    };

    Ok(quote! {
        impl crate::size_estimation::SizeEstimation for #name {
            fn estimated_cdr_size(&self) -> usize {
                #size_decl
                #( #padding_slack #field_size_exprs )*
                size
            }
        }

        impl #name {
            /// Get an accurate estimate of the serialized CDR size.
            ///
            /// This implementation accounts for dynamic fields (Vec, String, ZBuf)
            /// and provides a conservative but accurate upper bound for SHM allocation.
            pub fn estimated_serialized_size(&self) -> usize {
                4 + crate::size_estimation::SizeEstimation::estimated_cdr_size(self)  // 4 for CDR header
            }
        }
    })
}

/// Generate size calculation expression for a single field
fn generate_field_size_expr(
    field: &Field,
    _source_package: &str,
    _ctx: &GenerationContext,
) -> Result<TokenStream> {
    let field_name = escape_field_name(&field.name);

    // Handle different field types
    if is_zbuf_field(field) {
        // ZBuf: 4 bytes length prefix + data length
        // Note: ros_z::ZBuf derefs to zenoh_buffers::ZBuf, so .len() works via Deref
        Ok(quote! {
            size += 4 + {
                use ::zenoh_buffers::buffer::Buffer;
                self.#field_name.len()
            };
        })
    } else if field.field_type.base_type == "string" {
        match &field.field_type.array {
            ArrayType::Single => {
                // Single string: 4 bytes length prefix + string length
                Ok(quote! {
                    size += 4 + self.#field_name.len();
                })
            }
            ArrayType::Unbounded | ArrayType::Bounded(_) => {
                // Vec<String>: 4 bytes vec length + (4 + len) for each string
                Ok(quote! {
                    size += 4;
                    for s in &self.#field_name {
                        size += 4 + s.len();
                    }
                })
            }
            ArrayType::Fixed(_) => {
                // Fixed array of strings
                Ok(quote! {
                    for s in &self.#field_name {
                        size += 4 + s.len();
                    }
                })
            }
        }
    } else if is_primitive_type(&field.field_type.base_type) {
        // Primitive type
        let elem_size = get_primitive_size(&field.field_type.base_type)?;

        match &field.field_type.array {
            ArrayType::Single => {
                // Single primitive
                Ok(quote! {
                    size += #elem_size;
                })
            }
            ArrayType::Unbounded | ArrayType::Bounded(_) => {
                // Vec<primitive>: 4 bytes length + elements
                if elem_size == 1 {
                    // Optimization: for byte arrays, no need to multiply by 1
                    Ok(quote! {
                        size += 4 + self.#field_name.len();
                    })
                } else {
                    Ok(quote! {
                        size += 4 + (self.#field_name.len() * #elem_size);
                    })
                }
            }
            ArrayType::Fixed(n) => {
                // Fixed array: just the elements
                let total_size = n * elem_size;
                Ok(quote! {
                    size += #total_size;
                })
            }
        }
    } else {
        // Nested message type (anything not primitive or string)
        match &field.field_type.array {
            ArrayType::Single => {
                // Single nested message
                Ok(quote! {
                    size += crate::size_estimation::SizeEstimation::estimated_cdr_size(&self.#field_name);
                })
            }
            ArrayType::Unbounded | ArrayType::Bounded(_) => {
                // Vec<NestedType>
                Ok(quote! {
                    size += 4;
                    for item in &self.#field_name {
                        size += crate::size_estimation::SizeEstimation::estimated_cdr_size(item);
                    }
                })
            }
            ArrayType::Fixed(_) => {
                // Fixed array of nested messages
                Ok(quote! {
                    for item in &self.#field_name {
                        size += crate::size_estimation::SizeEstimation::estimated_cdr_size(item);
                    }
                })
            }
        }
    }
}

/// Check if a type is a primitive
fn is_primitive_type(base_type: &str) -> bool {
    matches!(
        base_type,
        "bool"
            | "byte"
            | "uint8"
            | "u8"
            | "int8"
            | "i8"
            | "char"
            | "uint16"
            | "u16"
            | "int16"
            | "i16"
            | "wchar"
            | "uint32"
            | "u32"
            | "int32"
            | "i32"
            | "float32"
            | "f32"
            | "uint64"
            | "u64"
            | "int64"
            | "i64"
            | "float64"
            | "f64"
    )
}

/// Get the size in bytes of a primitive type
fn get_primitive_size(base_type: &str) -> Result<usize> {
    Ok(match base_type {
        "bool" | "byte" | "uint8" | "u8" | "int8" | "i8" | "char" => 1,
        "uint16" | "u16" | "int16" | "i16" | "wchar" => 2,
        "uint32" | "u32" | "int32" | "i32" | "float32" | "f32" => 4,
        "uint64" | "u64" | "int64" | "i64" | "float64" | "f64" => 8,
        _ => bail!("Unknown primitive type: {}", base_type),
    })
}

/// Generate service type implementation
pub fn generate_service_impl(srv: &ResolvedService) -> Result<TokenStream> {
    let name = format_ident!("{}", srv.parsed.name);
    let request_type = format_ident!("{}Request", srv.parsed.name);
    let response_type = format_ident!("{}Response", srv.parsed.name);
    let service_type_name = srv.descriptor.type_name.as_str();
    let hash_str = srv.schema_hash.to_hash_string();

    Ok(quote! {
        pub struct #name;

        impl ::ros_z::msg::Service for #name {
            type Request = super::#request_type;
            type Response = super::#response_type;
        }

        impl ::ros_z::ServiceTypeInfo for #name {
            fn service_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    #service_type_name,
                    ::ros_z::entity::SchemaHash::from_hash_string(#hash_str)
                        .expect("invalid hash")
                )
            }
        }
    })
}

/// Generate action type implementation
/// Actions are generated similarly to services - the struct is at the root level,
/// while Goal/Result/Feedback types are in the package module
pub fn generate_action_impl(action: &crate::types::ResolvedAction) -> Result<TokenStream> {
    let name = format_ident!("{}", action.parsed.name);
    let goal_type = format_ident!("{}Goal", action.parsed.name);
    let result_type = format_ident!("{}Result", action.parsed.name);
    let feedback_type = format_ident!("{}Feedback", action.parsed.name);
    let action_type_name = action.descriptor.type_name.as_str();
    let hash_str = action.schema_hash.to_hash_string();

    // Native action protocol service/message names.
    let send_goal_type_name = format!("{}::{}SendGoal", action.parsed.package, action.parsed.name);
    let get_result_type_name =
        format!("{}::{}GetResult", action.parsed.package, action.parsed.name);
    let feedback_msg_type_name = format!(
        "{}::{}FeedbackMessage",
        action.parsed.package, action.parsed.name
    );

    // Get schema hashes from resolved action service hashes (not the Goal/Result/Feedback hashes)
    let send_goal_hash_str = action.send_goal_hash.to_hash_string();
    let get_result_hash_str = action.get_result_hash.to_hash_string();
    let feedback_hash_str = action.feedback_message_hash.to_hash_string();
    let cancel_goal_hash_str = action.cancel_goal_hash.to_hash_string();
    let status_hash_str = action.status_hash.to_hash_string();
    Ok(quote! {
        pub struct #name;

        impl ::ros_z::action::Action for #name {
            type Goal = super::#goal_type;
            type Result = super::#result_type;
            type Feedback = super::#feedback_type;

            fn name() -> &'static str {
                #action_type_name
            }

            fn send_goal_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    #send_goal_type_name,
                    ::ros_z::entity::SchemaHash::from_hash_string(#send_goal_hash_str)
                        .expect("invalid hash")
                )
            }

            fn get_result_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    #get_result_type_name,
                    ::ros_z::entity::SchemaHash::from_hash_string(#get_result_hash_str)
                        .expect("invalid hash")
                )
            }

            fn cancel_goal_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    "ros_z::action::CancelGoal",
                    ::ros_z::entity::SchemaHash::from_hash_string(#cancel_goal_hash_str)
                        .expect("invalid hash")
                )
            }

            fn feedback_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    #feedback_msg_type_name,
                    ::ros_z::entity::SchemaHash::from_hash_string(#feedback_hash_str)
                        .expect("invalid hash")
                )
            }

            fn status_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    "ros_z::action::GoalStatusArray",
                    ::ros_z::entity::SchemaHash::from_hash_string(#status_hash_str)
                        .expect("invalid hash")
                )
            }
        }

        impl ::ros_z::ActionTypeInfo for #name {
            fn action_type_info() -> ::ros_z::entity::TypeInfo {
                ::ros_z::entity::TypeInfo::with_hash(
                    #action_type_name,
                    ::ros_z::entity::SchemaHash::from_hash_string(#hash_str)
                        .expect("invalid hash")
                )
            }
        }
    })
}

/// Check if a field uses ZBuf (uint8[] or byte[])
fn is_zbuf_field(field: &Field) -> bool {
    matches!(
        (field.field_type.base_type.as_str(), &field.field_type.array),
        ("uint8" | "byte" | "u8", ArrayType::Unbounded)
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ros_z_schema::{FieldDef, FieldPrimitive, FieldShape, SchemaBundle, StructDef, TypeDef};

    use super::*;
    use crate::types::{Field, ParsedMessage, SchemaHash};

    fn build_schema(root: &str, fields: Vec<FieldDef>) -> SchemaBundle {
        SchemaBundle::builder(root)
            .definition(root, TypeDef::Struct(StructDef { fields }))
            .build_unchecked()
    }

    #[test]
    fn test_is_zbuf_field() {
        let field = Field {
            name: "data".to_string(),
            field_type: FieldType {
                base_type: "uint8".to_string(),
                package: None,
                array: ArrayType::Unbounded,
                string_bound: None,
            },
            default: None,
        };
        assert!(is_zbuf_field(&field));

        let field = Field {
            name: "data".to_string(),
            field_type: FieldType {
                base_type: "byte".to_string(),
                package: None,
                array: ArrayType::Unbounded,
                string_bound: None,
            },
            default: None,
        };
        assert!(is_zbuf_field(&field));

        let field = Field {
            name: "data".to_string(),
            field_type: FieldType {
                base_type: "int32".to_string(),
                package: None,
                array: ArrayType::Unbounded,
                string_bound: None,
            },
            default: None,
        };
        assert!(!is_zbuf_field(&field));
    }

    #[test]
    fn test_generate_simple_message() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Simple".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![Field {
                    name: "value".to_string(),
                    field_type: FieldType {
                        base_type: "int32".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::Simple",
                vec![FieldDef::new(
                    "value",
                    FieldShape::Primitive(FieldPrimitive::I32),
                )],
            ),
            schema_hash: SchemaHash([0u8; 32]),
            definition: String::new(),
        };

        let result = generate_message_impl(&msg);
        assert!(result.is_ok());

        let tokens = result.unwrap();
        let code = tokens.to_string();

        // Should contain struct definition
        assert!(code.contains("struct Simple"));
        // Should contain field
        assert!(code.contains("pub value"));
        // Should derive Serialize/Deserialize (no ZBuf)
        assert!(code.contains("Serialize"));
        assert!(code.contains("Deserialize"));
        // Should have Message
        assert!(code.contains("Message"));
        // Should provide optional runtime schema hook
        assert!(code.contains("schema"));
        assert!(code.contains("MessageSchema :: builder"));
        assert!(code.contains("FieldType :: Int32"));
    }

    #[test]
    fn test_generate_message_impl_uses_canonical_schema_fields() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "SchemaBacked".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![Field {
                    name: "stale".to_string(),
                    field_type: FieldType {
                        base_type: "bool".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::SchemaBacked",
                vec![FieldDef::new(
                    "value",
                    FieldShape::Primitive(FieldPrimitive::I32),
                )],
            ),
            schema_hash: SchemaHash([0u8; 32]),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains("pub value"));
        assert!(code.contains("FieldType :: Int32"));
        assert!(!code.contains("pub stale"));
        assert!(!code.contains("FieldType :: Bool"));
    }

    #[test]
    fn test_generate_char_field_uses_u8_semantics() {
        let field_type = FieldType {
            base_type: "char".to_string(),
            package: None,
            array: ArrayType::Single,
            string_bound: None,
        };

        let field_tokens = generate_field_type_tokens(&field_type, "test_msgs").to_string();
        let const_tokens = generate_constant_type("char").to_string();

        assert_eq!(field_tokens, "u8");
        assert_eq!(const_tokens, "u8");
    }

    #[test]
    fn test_size_estimation_includes_per_field_alignment_padding_upper_bound() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "MixedAlignment".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![
                    Field {
                        name: "small".to_string(),
                        field_type: FieldType {
                            base_type: "uint8".to_string(),
                            package: None,
                            array: ArrayType::Single,
                            string_bound: None,
                        },
                        default: None,
                    },
                    Field {
                        name: "large".to_string(),
                        field_type: FieldType {
                            base_type: "uint64".to_string(),
                            package: None,
                            array: ArrayType::Single,
                            string_bound: None,
                        },
                        default: None,
                    },
                ],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::MixedAlignment",
                vec![
                    FieldDef::new("small", FieldShape::Primitive(FieldPrimitive::U8)),
                    FieldDef::new("large", FieldShape::Primitive(FieldPrimitive::U64)),
                ],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains("size += 7usize ;"));
        assert!(!code.contains("size + 16"));
    }

    #[test]
    fn test_generate_signed_numeric_constants_do_not_panic() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Constants".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![
                    crate::types::Constant {
                        name: "NEGATIVE_ONE".to_string(),
                        const_type: "int32".to_string(),
                        value: "-1".to_string(),
                    },
                    crate::types::Constant {
                        name: "NEGATIVE_FLOAT".to_string(),
                        const_type: "float32".to_string(),
                        value: "-1.5".to_string(),
                    },
                ],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema("test_msgs::Constants", vec![]),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains("pub const NEGATIVE_ONE : i32 = - 1"));
        assert!(code.contains("pub const NEGATIVE_FLOAT : f32 = - 1.5"));
    }

    #[test]
    fn test_generate_message_impl_returns_error_for_malformed_constant() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Constants".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![crate::types::Constant {
                    name: "BROKEN".to_string(),
                    const_type: "int32".to_string(),
                    value: "abc".to_string(),
                }],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema("test_msgs::Constants", vec![]),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let err = generate_message_impl(&msg).unwrap_err();

        assert!(err.to_string().contains("invalid numeric constant `abc`"));
    }

    #[test]
    fn test_compute_plain_types_rejects_mixed_alignment_with_nested_plain_struct() {
        let inner = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Inner".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::Inner",
                vec![FieldDef::new(
                    "value",
                    FieldShape::Primitive(FieldPrimitive::U64),
                )],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };
        let outer = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Outer".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::Outer",
                vec![
                    FieldDef::new(
                        "inner",
                        FieldShape::Named(ros_z_schema::TypeName::new("test_msgs::Inner").unwrap()),
                    ),
                    FieldDef::new("flag", FieldShape::Primitive(FieldPrimitive::U8)),
                ],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let plain = compute_plain_types(&[inner, outer]).unwrap();

        assert!(plain.contains("test_msgs::Inner"));
        assert!(!plain.contains("test_msgs::Outer"));
    }

    #[test]
    fn test_compute_plain_types_rejects_large_fixed_arrays() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Covariance".to_string(),
                package: "geometry_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "geometry_msgs::Covariance",
                vec![FieldDef::new(
                    "covariance",
                    FieldShape::Array {
                        element: Box::new(FieldShape::Primitive(FieldPrimitive::F64)),
                        length: 36,
                    },
                )],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let plain = compute_plain_types(&[msg]).unwrap();

        assert!(!plain.contains("geometry_msgs::Covariance"));
    }

    #[test]
    fn test_bounded_sequence_codegen_enforces_bounds_on_read_and_write() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "BoundedNumbers".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![Field {
                    name: "values".to_string(),
                    field_type: FieldType {
                        base_type: "int32".to_string(),
                        package: None,
                        array: ArrayType::Bounded(4),
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::BoundedNumbers",
                vec![FieldDef::new(
                    "values",
                    FieldShape::BoundedSequence {
                        element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                        maximum_length: 4,
                    },
                )],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains("self . values . len () <= 4"));
        assert!(code.contains("__count > 4"));
    }

    #[test]
    fn test_generate_message_impl_uses_canonical_field_defaults() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "DefaultsFromSchema".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::DefaultsFromSchema",
                vec![
                    FieldDef::new("enabled", FieldShape::Primitive(FieldPrimitive::Bool))
                        .with_default(ros_z_schema::LiteralValue::Bool(true)),
                    FieldDef::new("label", FieldShape::String)
                        .with_default(ros_z_schema::LiteralValue::String("ready".to_string())),
                ],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains(":: smart_default :: SmartDefault"));
        assert!(code.contains("# [default (_code = \"true\")]"));
        assert!(code.contains("# [default (_code = \"\\\"ready\\\".to_string()\")]"));
    }

    #[test]
    fn test_generate_message_impl_uses_typed_default_code_for_narrow_numeric_fields() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "TypedDefaults".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::TypedDefaults",
                vec![
                    FieldDef::new("offset", FieldShape::Primitive(FieldPrimitive::I8))
                        .with_default(ros_z_schema::LiteralValue::Int(-2)),
                    FieldDef::new("mode", FieldShape::Primitive(FieldPrimitive::U8))
                        .with_default(ros_z_schema::LiteralValue::UInt(0)),
                    FieldDef::new("scale", FieldShape::Primitive(FieldPrimitive::F64))
                        .with_default(ros_z_schema::LiteralValue::Float64(1.0)),
                ],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let code = generate_message_impl(&msg).unwrap().to_string();

        assert!(code.contains("# [default (_code = \"-2i8\")]"));
        assert!(code.contains("# [default (_code = \"0u8\")]"));
        assert!(code.contains("# [default (_code = \"1f64\")]"));
    }

    #[test]
    fn test_generate_message_impl_returns_error_for_unprojectable_schema() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "OptionalField".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::OptionalField",
                vec![FieldDef::new(
                    "maybe_value",
                    FieldShape::Optional {
                        element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                    },
                )],
            ),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        };

        let err = generate_message_impl(&msg).unwrap_err();

        assert!(
            err.to_string()
                .contains("optional fields are not supported")
        );
    }

    #[test]
    fn test_generate_byte_array_message() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Image".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![
                    Field {
                        name: "width".to_string(),
                        field_type: FieldType {
                            base_type: "uint32".to_string(),
                            package: None,
                            array: ArrayType::Single,
                            string_bound: None,
                        },
                        default: None,
                    },
                    Field {
                        name: "data".to_string(),
                        field_type: FieldType {
                            base_type: "uint8".to_string(),
                            package: None,
                            array: ArrayType::Unbounded,
                            string_bound: None,
                        },
                        default: None,
                    },
                ],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::Image",
                vec![
                    FieldDef::new("width", FieldShape::Primitive(FieldPrimitive::U32)),
                    FieldDef::new(
                        "data",
                        FieldShape::Sequence {
                            element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                        },
                    ),
                ],
            ),
            schema_hash: SchemaHash([0u8; 32]),
            definition: String::new(),
        };

        let result = generate_message_impl(&msg);
        assert!(result.is_ok());

        let tokens = result.unwrap();
        let code = tokens.to_string();

        // Should contain ros_z::ZBuf field (wrapper with optimized serde)
        assert!(code.contains("ros_z :: ZBuf"));
        // Should use derived Serialize/Deserialize (ZBuf wrapper implements these traits)
        assert!(code.contains("derive"));
    }

    #[test]
    fn test_generate_service() {
        let request = ParsedMessage {
            name: "AddTwoIntsRequest".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![
                Field {
                    name: "a".to_string(),
                    field_type: FieldType {
                        base_type: "int64".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
                Field {
                    name: "b".to_string(),
                    field_type: FieldType {
                        base_type: "int64".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
            ],
            constants: vec![],
            source: String::new(),
            path: PathBuf::new(),
        };

        let response = ParsedMessage {
            name: "AddTwoIntsResponse".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![Field {
                name: "sum".to_string(),
                field_type: FieldType {
                    base_type: "int64".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: String::new(),
            path: PathBuf::new(),
        };

        let srv = ResolvedService {
            parsed: crate::types::ParsedService {
                name: "AddTwoInts".to_string(),
                package: "test_msgs".to_string(),
                request: request.clone(),
                response: response.clone(),
                source: String::new(),
                path: PathBuf::new(),
            },
            request: ResolvedMessage {
                parsed: request,
                schema: build_schema(
                    "test_msgs::AddTwoIntsRequest",
                    vec![
                        FieldDef::new("a", FieldShape::Primitive(FieldPrimitive::I64)),
                        FieldDef::new("b", FieldShape::Primitive(FieldPrimitive::I64)),
                    ],
                ),
                schema_hash: SchemaHash([0u8; 32]),
                definition: String::new(),
            },
            response: ResolvedMessage {
                parsed: response,
                schema: build_schema(
                    "test_msgs::AddTwoIntsResponse",
                    vec![FieldDef::new(
                        "sum",
                        FieldShape::Primitive(FieldPrimitive::I64),
                    )],
                ),
                schema_hash: SchemaHash([0u8; 32]),
                definition: String::new(),
            },
            descriptor: ros_z_schema::ServiceDef::new(
                "test_msgs::AddTwoInts",
                "test_msgs::AddTwoIntsRequest",
                "test_msgs::AddTwoIntsResponse",
            )
            .unwrap(),
            schema_hash: SchemaHash([0u8; 32]),
        };

        let request_impl = generate_message_impl(&srv.request).unwrap();
        let response_impl = generate_message_impl(&srv.response).unwrap();
        let result = generate_service_impl(&srv);
        assert!(result.is_ok());

        let tokens = result.unwrap();
        let code = quote::quote! {
            #request_impl
            #response_impl
            #tokens
        }
        .to_string();

        assert!(code.contains("struct AddTwoInts"));
        assert!(code.contains("ServiceTypeInfo"));
        assert!(code.contains("AddTwoIntsRequest"));
        assert!(code.contains("AddTwoIntsResponse"));
        assert!(code.contains("test_msgs::AddTwoInts"));
        assert!(code.contains("test_msgs::AddTwoIntsRequest"));
        assert!(code.contains("test_msgs::AddTwoIntsResponse"));
    }

    #[test]
    fn test_generate_schema_for_nested_message_uses_field_type_info_hook() {
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                name: "StampedPoint".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![Field {
                    name: "point".to_string(),
                    field_type: FieldType {
                        base_type: "Point".to_string(),
                        package: Some("geometry_msgs".to_string()),
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: String::new(),
                path: PathBuf::new(),
            },
            schema: build_schema(
                "test_msgs::StampedPoint",
                vec![FieldDef::new(
                    "point",
                    FieldShape::Named(ros_z_schema::TypeName::new("geometry_msgs::Point").unwrap()),
                )],
            ),
            schema_hash: SchemaHash([0u8; 32]),
            definition: String::new(),
        };

        let ctx = GenerationContext::new(
            Some("ros_z_msgs".to_string()),
            std::iter::once("test_msgs".to_string()).collect(),
        );

        let tokens = generate_message_impl_with_context(&msg, &ctx).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("FieldTypeInfo"));
        assert!(code.contains("field_type"));
        assert!(code.contains("geometry_msgs :: Point"));
    }

    #[test]
    fn test_generate_action_impl_handles_missing_result_and_feedback() {
        let action = crate::types::ResolvedAction {
            parsed: crate::types::ParsedAction {
                name: "Navigate".to_string(),
                package: "test_msgs".to_string(),
                goal: ParsedMessage {
                    name: "NavigateGoal".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                result: ParsedMessage {
                    name: "NavigateResult".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                feedback: ParsedMessage {
                    name: "NavigateFeedback".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                source: String::new(),
                path: PathBuf::from("/tmp/action/Navigate.action"),
            },
            goal: ResolvedMessage {
                parsed: ParsedMessage {
                    name: "NavigateGoal".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                schema: build_schema("test_msgs::NavigateGoal", vec![]),
                schema_hash: SchemaHash::zero(),
                definition: String::new(),
            },
            result: ResolvedMessage {
                parsed: ParsedMessage {
                    name: "NavigateResult".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                schema: build_schema("test_msgs::NavigateResult", vec![]),
                schema_hash: SchemaHash::zero(),
                definition: String::new(),
            },
            feedback: ResolvedMessage {
                parsed: ParsedMessage {
                    name: "NavigateFeedback".to_string(),
                    package: "test_msgs".to_string(),
                    fields: vec![],
                    constants: vec![],
                    source: String::new(),
                    path: PathBuf::from("/tmp/action/Navigate.action"),
                },
                schema: build_schema("test_msgs::NavigateFeedback", vec![]),
                schema_hash: SchemaHash::zero(),
                definition: String::new(),
            },
            descriptor: ros_z_schema::ActionDef::new(
                "test_msgs::Navigate",
                "test_msgs::NavigateGoal",
                "test_msgs::NavigateResult",
                "test_msgs::NavigateFeedback",
            )
            .unwrap(),
            schema_hash: SchemaHash::zero(),
            send_goal_hash: SchemaHash::zero(),
            get_result_hash: SchemaHash::zero(),
            feedback_message_hash: SchemaHash::zero(),
            cancel_goal_hash: SchemaHash::zero(),
            status_hash: SchemaHash::zero(),
        };

        let goal_impl = generate_message_impl(&action.goal).unwrap();
        let result_impl = generate_message_impl(&action.result).unwrap();
        let feedback_impl = generate_message_impl(&action.feedback).unwrap();
        let action_impl = generate_action_impl(&action).unwrap();
        let code = quote::quote! {
            #goal_impl
            #result_impl
            #feedback_impl
            #action_impl
        }
        .to_string();

        assert!(!code.contains("pub struct NavigateResult ;"));
        assert!(!code.contains("pub struct NavigateFeedback ;"));
        assert!(code.contains("type Result = super :: NavigateResult"));
        assert!(code.contains("type Feedback = super :: NavigateFeedback"));
        assert!(code.contains("test_msgs::Navigate"));
        assert!(code.contains("test_msgs::NavigateGoal"));
        assert!(code.contains("test_msgs::NavigateResult"));
        assert!(code.contains("test_msgs::NavigateFeedback"));
        assert!(code.contains("test_msgs::NavigateSendGoal"));
        assert!(code.contains("test_msgs::NavigateGetResult"));
        assert!(code.contains("test_msgs::NavigateFeedbackMessage"));
        assert!(code.contains("impl :: ros_z :: action :: Action for Navigate"));
        assert!(!code.contains("fn cdr_decode < BO , B >"));
    }
}
