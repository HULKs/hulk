use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use color_eyre::eyre::{Result, bail};
use ros_z_schema::{
    FieldDef, FieldPrimitive, FieldShape, SchemaBundle, SchemaBundleBuilder, StructDef, TypeDef,
    TypeName,
};

use crate::{
    hashing::{
        build_schema_bundle, calculate_action_hash, calculate_message_hash, calculate_service_hash,
        is_primitive_type,
    },
    parser::msg::parse_msg_file,
    types::{ParsedMessage, ParsedService, ResolvedMessage, ResolvedService},
};

pub struct Resolver {
    /// Map of package/name -> schema bundles for resolved types
    schema_bundles: BTreeMap<String, ros_z_schema::SchemaBundle>,
    /// Map of package/name -> ResolvedMessage for fully resolved messages
    resolved_messages: HashMap<String, ResolvedMessage>,
    /// Parsed external messages used for canonical nested definition expansion.
    external_messages: HashMap<String, ParsedMessage>,
    /// External packages available from bundled ros_z_msgs interface sources.
    external_packages: HashSet<String>,
}

impl Resolver {
    /// Create a new resolver
    pub fn new() -> Self {
        Self {
            schema_bundles: BTreeMap::new(),
            resolved_messages: HashMap::new(),
            external_messages: HashMap::new(),
            external_packages: HashSet::new(),
        }
    }

    /// Create a resolver with external packages
    /// External packages are treated as already resolved (types come from another crate)
    pub fn with_external_packages(external_packages: HashSet<String>) -> Result<Self> {
        let mut resolver = Self::new();
        resolver.external_packages = external_packages;
        Ok(resolver)
    }

    /// Resolve a list of messages, calculating their schema hashes and dependencies
    pub fn resolve_messages(
        &mut self,
        messages: Vec<ParsedMessage>,
    ) -> Result<Vec<ResolvedMessage>> {
        let mut queue = VecDeque::from(messages);
        let mut resolved_order = Vec::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000;
        let mut last_queue_size = queue.len();
        let mut stagnant_iterations = 0;

        while let Some(msg) = queue.pop_front() {
            if iterations >= MAX_ITERATIONS {
                // Provide detailed error with stuck messages
                let stuck_messages: Vec<String> = queue
                    .iter()
                    .take(10)
                    .map(|m| format!("{}/{}", m.package, m.name))
                    .collect();

                bail!(
                    "Exceeded maximum iterations ({}) - possible circular dependency or missing type.\n\
                     Still unresolved: {} messages\n\
                     First 10 stuck messages: {:?}",
                    MAX_ITERATIONS,
                    queue.len() + 1,
                    stuck_messages
                );
            }
            iterations += 1;

            self.ensure_external_dependencies_loaded(&msg)?;

            // Check if all dependencies are resolved
            if self.all_deps_resolved(&msg) {
                let resolved = self.resolve_message(msg)?;
                let key = self.message_key(&resolved.parsed);

                self.schema_bundles
                    .insert(key.clone(), resolved.schema.clone());

                resolved_order.push(key.clone());
                self.resolved_messages.insert(key, resolved);

                // Reset stagnation counter on progress
                stagnant_iterations = 0;
            } else {
                // Push back to queue to try again later
                queue.push_back(msg);
            }

            // Detect stagnation (no progress being made)
            if queue.len() == last_queue_size {
                stagnant_iterations += 1;
                if stagnant_iterations > 100 {
                    // We've gone through the queue 100 times without resolving anything
                    let stuck_messages: Vec<String> = queue
                        .iter()
                        .take(10)
                        .map(|m| {
                            let unresolved_deps: Vec<String> = m
                                .fields
                                .iter()
                                .filter(|f| {
                                    !is_primitive_type(&f.field_type.base_type)
                                        && !self.is_type_resolved(
                                            &f.field_type.package,
                                            &f.field_type.base_type,
                                        )
                                })
                                .map(|f| format!("{:?}", f.field_type))
                                .collect();
                            format!("{}/{} (missing: {:?})", m.package, m.name, unresolved_deps)
                        })
                        .collect();

                    bail!(
                        "Dependency resolution stalled after {} iterations.\n\
                         {} messages cannot be resolved.\n\
                         First 10 stuck messages with dependencies:\n{}",
                        iterations,
                        queue.len(),
                        stuck_messages.join("\n")
                    );
                }
            } else {
                last_queue_size = queue.len();
                stagnant_iterations = 0;
            }
        }

        Ok(resolved_order
            .into_iter()
            .filter_map(|key| self.resolved_messages.get(&key).cloned())
            .collect())
    }

    /// Resolve a single service (Request + Response + Event)
    pub fn resolve_service(&mut self, srv: ParsedService) -> Result<ResolvedService> {
        // First resolve request and response as standalone messages
        self.ensure_external_dependencies_loaded(&srv.request)?;
        self.ensure_external_dependencies_loaded(&srv.response)?;
        let request = self.resolve_message(srv.request.clone())?;
        self.schema_bundles
            .insert(self.message_key(&srv.request), request.schema.clone());
        self.resolved_messages
            .insert(self.message_key(&srv.request), request.clone());

        self.ensure_external_dependencies_loaded(&srv.response)?;
        let response = self.resolve_message(srv.response.clone())?;
        self.schema_bundles
            .insert(self.message_key(&srv.response), response.schema.clone());
        self.resolved_messages
            .insert(self.message_key(&srv.response), response.clone());

        let descriptor = ros_z_schema::ServiceDef::new(
            format!("{}::{}", srv.package, srv.name),
            request.schema.root.as_str(),
            response.schema.root.as_str(),
        )?;
        let schema_hash = calculate_service_hash(&descriptor);

        Ok(ResolvedService {
            parsed: srv,
            request,
            response,
            descriptor,
            schema_hash,
        })
    }

    /// Resolve multiple services
    pub fn resolve_services(
        &mut self,
        services: Vec<ParsedService>,
    ) -> Result<Vec<ResolvedService>> {
        services
            .into_iter()
            .map(|srv| self.resolve_service(srv))
            .collect()
    }

    /// Resolve a single action (Goal + Result + Feedback)
    pub fn resolve_action(
        &mut self,
        action: crate::types::ParsedAction,
    ) -> Result<crate::types::ResolvedAction> {
        use crate::types::ResolvedAction;

        // Resolve goal, result, and feedback as standalone messages
        self.ensure_external_dependencies_loaded(&action.goal)?;
        let goal = self.resolve_message(action.goal.clone())?;
        self.schema_bundles
            .insert(self.message_key(&action.goal), goal.schema.clone());
        self.resolved_messages
            .insert(self.message_key(&action.goal), goal.clone());

        self.ensure_external_dependencies_loaded(&action.result)?;
        let result = self.resolve_message(action.result.clone())?;
        self.schema_bundles
            .insert(self.message_key(&action.result), result.schema.clone());
        self.resolved_messages
            .insert(self.message_key(&action.result), result.clone());

        self.ensure_external_dependencies_loaded(&action.feedback)?;
        let feedback = self.resolve_message(action.feedback.clone())?;
        self.schema_bundles
            .insert(self.message_key(&action.feedback), feedback.schema.clone());
        self.resolved_messages
            .insert(self.message_key(&action.feedback), feedback.clone());

        let descriptor = ros_z_schema::ActionDef::new(
            format!("{}::{}", action.package, action.name),
            goal.schema.root.as_str(),
            result.schema.root.as_str(),
            feedback.schema.root.as_str(),
        )?;
        let schema_hash = calculate_action_hash(&descriptor);

        // Calculate schema hashes for action protocol services/messages
        // These follow ROS2 action protocol structure
        let send_goal_hash = self.calculate_send_goal_hash(&action, &goal)?;
        let get_result_hash = self.calculate_get_result_hash(&action, &result)?;
        let feedback_message_hash = self.calculate_feedback_message_hash(&action, &feedback)?;

        // Calculate native action protocol schema hashes shared by all actions.
        let cancel_goal_hash = self.calculate_cancel_goal_hash()?;
        let status_hash = self.calculate_status_hash()?;

        Ok(ResolvedAction {
            parsed: action.clone(),
            goal,
            result,
            feedback,
            descriptor,
            schema_hash,
            send_goal_hash,
            get_result_hash,
            feedback_message_hash,
            cancel_goal_hash,
            status_hash,
        })
    }

    /// Resolve multiple actions
    pub fn resolve_actions(
        &mut self,
        actions: Vec<crate::types::ParsedAction>,
    ) -> Result<Vec<crate::types::ResolvedAction>> {
        actions
            .into_iter()
            .map(|action| self.resolve_action(action))
            .collect()
    }

    /// Check if all dependencies for a message are resolved
    fn all_deps_resolved(&self, msg: &ParsedMessage) -> bool {
        msg.fields.iter().all(|field| {
            is_primitive_type(&field.field_type.base_type)
                || self.is_type_resolved_in_context(
                    &field.field_type.package,
                    &field.field_type.base_type,
                    &msg.package,
                )
        })
    }

    /// Check if a type is already resolved
    fn is_type_resolved(&self, package: &Option<String>, base_type: &str) -> bool {
        if let Some(pkg) = package {
            let key = format!("{}/{}", pkg, base_type);
            self.schema_bundles.contains_key(&key)
        } else {
            false
        }
    }

    /// Check if a type is resolved, considering same-package references and external packages
    fn is_type_resolved_in_context(
        &self,
        package: &Option<String>,
        base_type: &str,
        source_package: &str,
    ) -> bool {
        if let Some(pkg) = package {
            let key = format!("{}/{}", pkg, base_type);
            self.schema_bundles.contains_key(&key)
        } else {
            // If package is None, assume it's in the same package as the source message
            let key = format!("{}/{}", source_package, base_type);
            self.schema_bundles.contains_key(&key)
        }
    }

    fn ensure_external_dependencies_loaded(&mut self, msg: &ParsedMessage) -> Result<()> {
        for field in &msg.fields {
            if is_primitive_type(&field.field_type.base_type) {
                continue;
            }

            let package = field.field_type.package.as_deref().unwrap_or(&msg.package);
            let key = format!("{package}/{}", field.field_type.base_type);
            if package == msg.package || self.schema_bundles.contains_key(&key) {
                continue;
            }

            if !self.external_packages.contains(package) {
                continue;
            }

            self.load_external_message_bundle(
                package,
                &field.field_type.base_type,
                &mut HashSet::new(),
            )?;
        }

        Ok(())
    }

    fn load_external_message_bundle(
        &mut self,
        package: &str,
        message_name: &str,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        let key = format!("{package}/{message_name}");
        if self.schema_bundles.contains_key(&key) {
            return Ok(());
        }
        if !self.external_packages.contains(package) {
            bail!("missing schema bundle for dependency `{key}`");
        }
        if !visiting.insert(key.clone()) {
            bail!("circular external dependency while loading `{key}`");
        }

        let path = bundled_interfaces_root()
            .join(package)
            .join("msg")
            .join(format!("{message_name}.msg"));
        if !path.is_file() {
            bail!(
                "missing bundled message source for external dependency `{key}` at `{}`",
                path.display()
            );
        }

        let parsed = parse_msg_file(&path, package)?;
        for field in &parsed.fields {
            if is_primitive_type(&field.field_type.base_type) {
                continue;
            }

            let dep_package = field.field_type.package.as_deref().unwrap_or(package);
            self.load_external_message_bundle(dep_package, &field.field_type.base_type, visiting)?;
        }

        let schema = build_schema_bundle(&parsed, &self.schema_bundles)?;
        let schema_hash = calculate_message_hash(&schema);
        self.schema_bundles.insert(key.clone(), schema.clone());
        self.external_messages.insert(key.clone(), parsed.clone());
        self.resolved_messages.insert(
            key.clone(),
            ResolvedMessage {
                parsed: parsed.clone(),
                schema,
                schema_hash,
                definition: String::new(),
            },
        );

        let definition = self.expand_definition(&parsed);
        if let Some(resolved) = self.resolved_messages.get_mut(&key) {
            resolved.definition = definition;
        }

        visiting.remove(&key);
        Ok(())
    }

    /// Resolve a single message
    fn resolve_message(&self, msg: ParsedMessage) -> Result<ResolvedMessage> {
        let schema = build_schema_bundle(&msg, &self.schema_bundles)?;
        let schema_hash = calculate_message_hash(&schema);

        // Expand full definition (include all nested types)
        let definition = self.expand_definition(&msg);

        Ok(ResolvedMessage {
            parsed: msg,
            schema,
            schema_hash,
            definition,
        })
    }

    /// Expand message definition to include all nested types
    fn expand_definition(&self, msg: &ParsedMessage) -> String {
        let mut definition = msg.source.clone();

        let mut visited = std::collections::BTreeSet::new();
        let mut pending: VecDeque<String> = msg
            .fields
            .iter()
            .filter(|field| !is_primitive_type(&field.field_type.base_type))
            .map(|field| {
                format!(
                    "{}/{}",
                    field.field_type.package.as_deref().unwrap_or(&msg.package),
                    field.field_type.base_type
                )
            })
            .collect();

        while let Some(nested_key) = pending.pop_front() {
            if !visited.insert(nested_key.clone()) {
                continue;
            }

            if let Some(resolved) = self.resolved_messages.get(&nested_key) {
                definition.push_str("\n\n");
                definition.push_str("================================================================================\n");
                definition.push_str(&format!("MSG: {}\n", nested_key));
                definition.push_str(&resolved.parsed.source);

                for field in &resolved.parsed.fields {
                    if is_primitive_type(&field.field_type.base_type) {
                        continue;
                    }

                    pending.push_back(format!(
                        "{}/{}",
                        field
                            .field_type
                            .package
                            .as_deref()
                            .unwrap_or(&resolved.parsed.package),
                        field.field_type.base_type
                    ));
                }
            } else if let Some(parsed) = self.external_messages.get(&nested_key) {
                definition.push_str("\n\n");
                definition.push_str("================================================================================\n");
                definition.push_str(&format!("MSG: {}\n", nested_key));
                definition.push_str(&parsed.source);

                for field in &parsed.fields {
                    if is_primitive_type(&field.field_type.base_type) {
                        continue;
                    }

                    pending.push_back(format!(
                        "{}/{}",
                        field
                            .field_type
                            .package
                            .as_deref()
                            .unwrap_or(&parsed.package),
                        field.field_type.base_type
                    ));
                }
            }
        }

        definition
    }

    /// Get the key for a message (package/name)
    fn message_key(&self, msg: &ParsedMessage) -> String {
        format!("{}/{}", msg.package, msg.name)
    }

    /// Calculate schema hash for action SendGoal service
    /// Request: goal_id (UUID) + goal
    /// Response: accepted (bool) + stamp (Time)
    fn calculate_send_goal_hash(
        &self,
        action: &crate::types::ParsedAction,
        _goal: &crate::types::ResolvedMessage,
    ) -> Result<crate::types::SchemaHash> {
        let descriptor = ros_z_schema::ServiceDef::new(
            format!("{}::{}SendGoal", action.package, action.name),
            format!("{}::{}SendGoalRequest", action.package, action.name),
            format!("{}::{}SendGoalResponse", action.package, action.name),
        )?;
        Ok(calculate_service_hash(&descriptor))
    }

    /// Calculate schema hash for action GetResult service
    /// Request: goal_id (UUID)
    /// Response: status (int8) + result
    fn calculate_get_result_hash(
        &self,
        action: &crate::types::ParsedAction,
        _result: &crate::types::ResolvedMessage,
    ) -> Result<crate::types::SchemaHash> {
        let descriptor = ros_z_schema::ServiceDef::new(
            format!("{}::{}GetResult", action.package, action.name),
            format!("{}::{}GetResultRequest", action.package, action.name),
            format!("{}::{}GetResultResponse", action.package, action.name),
        )?;
        Ok(calculate_service_hash(&descriptor))
    }

    /// Calculate schema hash for action feedback message
    fn calculate_feedback_message_hash(
        &self,
        action: &crate::types::ParsedAction,
        feedback: &crate::types::ResolvedMessage,
    ) -> Result<crate::types::SchemaHash> {
        let root = format!("{}::{}FeedbackMessage", action.package, action.name);
        let builder = SchemaBundle::builder(root.clone()).definition(
            root,
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "goal_id",
                        FieldShape::Named(TypeName::new("ros_z::action::GoalId")?),
                    ),
                    FieldDef::new(
                        "feedback",
                        FieldShape::Named(TypeName::new(feedback.schema.root.as_str())?),
                    ),
                ],
            }),
        );
        let builder = add_bundle_definitions(builder, &uuid_bundle()?);
        let builder = add_bundle_definitions(builder, &feedback.schema);
        let bundle = builder.build()?;
        Ok(calculate_message_hash(&bundle))
    }

    /// Calculate schema hash for the native CancelGoal service.
    fn calculate_cancel_goal_hash(&self) -> Result<crate::types::SchemaHash> {
        let descriptor = ros_z_schema::ServiceDef::new(
            "ros_z::action::CancelGoal",
            "ros_z::action::CancelGoalRequest",
            "ros_z::action::CancelGoalResponse",
        )?;
        Ok(calculate_service_hash(&descriptor))
    }

    /// Calculate schema hash for the native GoalStatusArray message.
    fn calculate_status_hash(&self) -> Result<crate::types::SchemaHash> {
        Ok(calculate_message_hash(&goal_status_array_bundle()?))
    }
}

fn bundled_interfaces_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ros-z-msgs/interfaces")
}

fn add_bundle_definitions(
    builder: SchemaBundleBuilder,
    bundle: &SchemaBundle,
) -> SchemaBundleBuilder {
    bundle
        .definitions()
        .iter()
        .fold(builder, |builder, (type_name, definition)| {
            builder.definition(type_name.as_str(), definition.clone())
        })
}

fn uuid_bundle() -> Result<SchemaBundle> {
    SchemaBundle::builder("ros_z::action::GoalId")
        .definition(
            "ros_z::action::GoalId",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "uuid",
                    FieldShape::Array {
                        element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                        length: 16,
                    },
                )],
            }),
        )
        .build()
        .map_err(Into::into)
}

fn time_bundle() -> Result<SchemaBundle> {
    SchemaBundle::builder("builtin_interfaces::Time")
        .definition(
            "builtin_interfaces::Time",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("sec", FieldShape::Primitive(FieldPrimitive::I32)),
                    FieldDef::new("nanosec", FieldShape::Primitive(FieldPrimitive::U32)),
                ],
            }),
        )
        .build()
        .map_err(Into::into)
}

fn goal_info_bundle() -> Result<SchemaBundle> {
    let uuid = uuid_bundle()?;
    let time = time_bundle()?;
    let builder = SchemaBundle::builder("ros_z::action::GoalInfo").definition(
        "ros_z::action::GoalInfo",
        TypeDef::Struct(StructDef {
            fields: vec![
                FieldDef::new(
                    "goal_id",
                    FieldShape::Named(TypeName::new("ros_z::action::GoalId")?),
                ),
                FieldDef::new(
                    "stamp",
                    FieldShape::Named(TypeName::new("builtin_interfaces::Time")?),
                ),
            ],
        }),
    );
    let builder = add_bundle_definitions(builder, &uuid);
    let builder = add_bundle_definitions(builder, &time);
    Ok(builder.build()?)
}

fn goal_status_array_bundle() -> Result<SchemaBundle> {
    let goal_info = goal_info_bundle()?;
    let builder = SchemaBundle::builder("ros_z::action::GoalStatusArray")
        .definition(
            "ros_z::action::GoalStatusArray",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "status_list",
                    FieldShape::Sequence {
                        element: Box::new(FieldShape::Named(TypeName::new(
                            "ros_z::action::GoalStatus",
                        )?)),
                    },
                )],
            }),
        )
        .definition(
            "ros_z::action::GoalStatus",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "goal_info",
                        FieldShape::Named(TypeName::new("ros_z::action::GoalInfo")?),
                    ),
                    FieldDef::new("status", FieldShape::Primitive(FieldPrimitive::I8)),
                ],
            }),
        );
    let builder = add_bundle_definitions(builder, &goal_info);
    Ok(builder.build()?)
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ros_z_schema::{FieldDef, FieldShape, SchemaBundle, StructDef, TypeDef, TypeName};

    use super::*;
    use crate::parser::action::parse_action;
    use crate::types::{ArrayType, Field, FieldType, schema_fields};

    fn parse_test_action(name: &str) -> crate::types::ParsedAction {
        let source = match name {
            "Fibonacci" => "int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence\n",
            other => panic!("unsupported test action: {other}"),
        };

        parse_action(
            source,
            name,
            "test_actions",
            &PathBuf::from(format!("/tmp/action/{name}.action")),
        )
        .unwrap()
    }

    fn bundle_from_fields(root: &str, fields: Vec<FieldDef>) -> SchemaBundle {
        SchemaBundle::builder(root)
            .definition(root, TypeDef::Struct(StructDef { fields }))
            .build()
            .unwrap()
    }

    fn uuid_bundle() -> SchemaBundle {
        bundle_from_fields(
            "ros_z::action::GoalId",
            vec![FieldDef::new(
                "uuid",
                FieldShape::Array {
                    element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                    length: 16,
                },
            )],
        )
    }

    fn time_bundle() -> SchemaBundle {
        bundle_from_fields(
            "builtin_interfaces::Time",
            vec![
                FieldDef::new("sec", FieldShape::Primitive(FieldPrimitive::I32)),
                FieldDef::new("nanosec", FieldShape::Primitive(FieldPrimitive::U32)),
            ],
        )
    }

    fn goal_info_bundle() -> SchemaBundle {
        let uuid = uuid_bundle();
        let time = time_bundle();
        SchemaBundle::builder("ros_z::action::GoalInfo")
            .definition(
                "ros_z::action::GoalInfo",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "goal_id",
                            FieldShape::Named(TypeName::new("ros_z::action::GoalId").unwrap()),
                        ),
                        FieldDef::new(
                            "stamp",
                            FieldShape::Named(TypeName::new("builtin_interfaces::Time").unwrap()),
                        ),
                    ],
                }),
            )
            .definition(uuid.root.as_str(), uuid.definitions[&uuid.root].clone())
            .definition(time.root.as_str(), time.definitions[&time.root].clone())
            .build()
            .unwrap()
    }

    fn goal_status_array_bundle() -> SchemaBundle {
        let uuid = uuid_bundle();
        let time = time_bundle();
        let goal_info = goal_info_bundle();
        SchemaBundle::builder("ros_z::action::GoalStatusArray")
            .definition(
                "ros_z::action::GoalStatusArray",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new(
                        "status_list",
                        FieldShape::Sequence {
                            element: Box::new(FieldShape::Named(
                                TypeName::new("ros_z::action::GoalStatus").unwrap(),
                            )),
                        },
                    )],
                }),
            )
            .definition(
                "ros_z::action::GoalStatus",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "goal_info",
                            FieldShape::Named(TypeName::new("ros_z::action::GoalInfo").unwrap()),
                        ),
                        FieldDef::new("status", FieldShape::Primitive(FieldPrimitive::I8)),
                    ],
                }),
            )
            .definition(
                goal_info.root.as_str(),
                goal_info.definitions[&goal_info.root].clone(),
            )
            .definition(uuid.root.as_str(), uuid.definitions[&uuid.root].clone())
            .definition(time.root.as_str(), time.definitions[&time.root].clone())
            .build()
            .unwrap()
    }

    #[test]
    fn test_resolver_new() {
        let resolver = Resolver::new();
        // Resolver starts empty - builtin types come from assets
        assert!(resolver.schema_bundles.is_empty());
        assert!(resolver.resolved_messages.is_empty());
    }

    #[test]
    fn with_external_packages_preloads_canonical_schema_bundles() {
        let msg = ParsedMessage {
            name: "PoseWrapper".to_string(),
            package: "my_test_msgs".to_string(),
            fields: vec![Field {
                name: "position".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: Some("geometry_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "geometry_msgs/Point position".to_string(),
            path: PathBuf::from("/tmp/my_test_msgs/msg/PoseWrapper.msg"),
        };

        let mut external_packages = HashSet::new();
        external_packages.insert("geometry_msgs".to_string());

        let mut resolver = Resolver::with_external_packages(external_packages).unwrap();
        let resolved = resolver.resolve_messages(vec![msg]).unwrap();
        let resolved_msg = resolved
            .iter()
            .find(|msg| msg.parsed.name == "PoseWrapper")
            .unwrap();

        assert!(
            resolved_msg
                .schema
                .definitions()
                .contains_key(&TypeName::new("geometry_msgs::Point").unwrap())
        );
    }

    #[test]
    fn expand_definition_includes_nested_external_bundle_dependencies() {
        let msg = ParsedMessage {
            name: "PoseWrapper".to_string(),
            package: "my_test_msgs".to_string(),
            fields: vec![Field {
                name: "header".to_string(),
                field_type: FieldType {
                    base_type: "Header".to_string(),
                    package: Some("std_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "std_msgs/Header header".to_string(),
            path: PathBuf::from("/tmp/my_test_msgs/msg/PoseWrapper.msg"),
        };

        let mut external_packages = HashSet::new();
        external_packages.insert("std_msgs".to_string());
        external_packages.insert("builtin_interfaces".to_string());

        let mut resolver = Resolver::with_external_packages(external_packages).unwrap();
        let resolved = resolver.resolve_messages(vec![msg]).unwrap();
        let resolved_msg = resolved
            .iter()
            .find(|msg| msg.parsed.name == "PoseWrapper")
            .unwrap();

        assert!(resolved_msg.definition.contains("MSG: std_msgs/Header"));
        assert!(
            resolved_msg
                .definition
                .contains("builtin_interfaces/Time stamp")
        );
        assert!(
            resolved_msg
                .definition
                .contains("MSG: builtin_interfaces/Time")
        );
        assert!(resolved_msg.definition.contains("uint32 nanosec"));
    }

    #[test]
    fn resolve_messages_allows_local_cross_package_dependencies() {
        let goal_info = ParsedMessage {
            name: "GoalInfo".to_string(),
            package: "test_action_protocol".to_string(),
            fields: vec![Field {
                name: "goal_id".to_string(),
                field_type: FieldType {
                    base_type: "UUID".to_string(),
                    package: Some("test_identifier_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "test_identifier_msgs/UUID goal_id".to_string(),
            path: PathBuf::from("/tmp/test_action_protocol/msg/GoalInfo.msg"),
        };
        let uuid = ParsedMessage {
            name: "UUID".to_string(),
            package: "test_identifier_msgs".to_string(),
            fields: vec![Field {
                name: "uuid".to_string(),
                field_type: FieldType {
                    base_type: "uint8".to_string(),
                    package: None,
                    array: ArrayType::Fixed(16),
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "uint8[16] uuid".to_string(),
            path: PathBuf::from("/tmp/test_identifier_msgs/msg/UUID.msg"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_messages(vec![goal_info, uuid]).unwrap();
        let goal_info = resolved
            .iter()
            .find(|message| {
                message.parsed.package == "test_action_protocol"
                    && message.parsed.name == "GoalInfo"
            })
            .unwrap();

        assert!(
            goal_info
                .schema
                .definitions()
                .contains_key(&TypeName::new("test_identifier_msgs::UUID").unwrap())
        );
    }

    #[test]
    fn resolve_service_handles_response_dependency_on_request_type() {
        let request = ParsedMessage {
            name: "LinkRequest".to_string(),
            package: "test_pkg".to_string(),
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
            source: "int32 value".to_string(),
            path: PathBuf::from("/tmp/srv/Link.srv"),
        };
        let response = ParsedMessage {
            name: "LinkResponse".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "request_copy".to_string(),
                field_type: FieldType {
                    base_type: "LinkRequest".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "LinkRequest request_copy".to_string(),
            path: PathBuf::from("/tmp/srv/Link.srv"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver
            .resolve_service(crate::types::ParsedService {
                name: "Link".to_string(),
                package: "test_pkg".to_string(),
                request,
                response,
                source: String::new(),
                path: PathBuf::from("/tmp/srv/Link.srv"),
            })
            .unwrap();

        assert!(
            resolved
                .response
                .schema
                .definitions()
                .contains_key(&TypeName::new("test_pkg::LinkRequest").unwrap())
        );
    }

    #[test]
    fn resolve_action_handles_component_dependency_order() {
        let action = crate::types::ParsedAction {
            name: "Link".to_string(),
            package: "test_pkg".to_string(),
            goal: ParsedMessage {
                name: "LinkGoal".to_string(),
                package: "test_pkg".to_string(),
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
                source: "int32 value".to_string(),
                path: PathBuf::from("/tmp/action/Link.action"),
            },
            result: ParsedMessage {
                name: "LinkResult".to_string(),
                package: "test_pkg".to_string(),
                fields: vec![Field {
                    name: "goal_copy".to_string(),
                    field_type: FieldType {
                        base_type: "LinkGoal".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: "LinkGoal goal_copy".to_string(),
                path: PathBuf::from("/tmp/action/Link.action"),
            },
            feedback: ParsedMessage {
                name: "LinkFeedback".to_string(),
                package: "test_pkg".to_string(),
                fields: vec![Field {
                    name: "result_copy".to_string(),
                    field_type: FieldType {
                        base_type: "LinkResult".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
                source: "LinkResult result_copy".to_string(),
                path: PathBuf::from("/tmp/action/Link.action"),
            },
            source: String::new(),
            path: PathBuf::from("/tmp/action/Link.action"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_action(action).unwrap();

        assert!(
            resolved
                .result
                .schema
                .definitions()
                .contains_key(&TypeName::new("test_pkg::LinkGoal").unwrap())
        );
        assert!(
            resolved
                .feedback
                .schema
                .definitions()
                .contains_key(&TypeName::new("test_pkg::LinkResult").unwrap())
        );
    }

    #[test]
    fn resolve_action_materializes_missing_result_and_feedback_as_zero_field_messages() {
        let action = parse_action(
            "int32 order\n---\n---\n",
            "Wait",
            "test_pkg",
            &PathBuf::from("/tmp/action/Wait.action"),
        )
        .unwrap();

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_action(action).unwrap();
        let result = &resolved.result;
        let feedback = &resolved.feedback;

        assert_eq!(result.schema.root.as_str(), "test_pkg::WaitResult");
        assert_eq!(schema_fields(result).unwrap().len(), 0);
        assert_eq!(feedback.schema.root.as_str(), "test_pkg::WaitFeedback");
        assert_eq!(schema_fields(feedback).unwrap().len(), 0);
        assert_eq!(resolved.descriptor.result.as_str(), "test_pkg::WaitResult");
        assert_eq!(
            resolved.descriptor.feedback.as_str(),
            "test_pkg::WaitFeedback"
        );
    }

    #[test]
    fn resolve_messages_returns_deterministic_resolution_order() {
        let dependent = ParsedMessage {
            name: "Pose".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "point".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "Point point".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Pose.msg"),
        };
        let independent = ParsedMessage {
            name: "Alpha".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![],
            constants: vec![],
            source: String::new(),
            path: PathBuf::from("/tmp/test_pkg/msg/Alpha.msg"),
        };
        let prerequisite = ParsedMessage {
            name: "Point".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "x".to_string(),
                field_type: FieldType {
                    base_type: "float64".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "float64 x".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Point.msg"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver
            .resolve_messages(vec![dependent, independent, prerequisite])
            .unwrap();
        let names: Vec<_> = resolved.into_iter().map(|msg| msg.parsed.name).collect();

        assert_eq!(names, vec!["Alpha", "Point", "Pose"]);
    }

    #[test]
    fn expand_definition_includes_same_package_nested_dependencies() {
        let point = ParsedMessage {
            name: "Point".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "x".to_string(),
                field_type: FieldType {
                    base_type: "float64".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "float64 x".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Point.msg"),
        };
        let pose = ParsedMessage {
            name: "Pose".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "point".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "Point point".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Pose.msg"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_messages(vec![point, pose]).unwrap();
        let pose = resolved
            .into_iter()
            .find(|msg| msg.parsed.name == "Pose")
            .unwrap();

        assert!(pose.definition.contains("MSG: test_pkg/Point"));
        assert!(pose.definition.contains("float64 x"));
    }

    #[test]
    fn expand_definition_does_not_duplicate_transitive_nested_sections() {
        let point = ParsedMessage {
            name: "Point".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "x".to_string(),
                field_type: FieldType {
                    base_type: "float64".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "float64 x".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Point.msg"),
        };
        let pose = ParsedMessage {
            name: "Pose".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "point".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "Point point".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Pose.msg"),
        };
        let wrapper = ParsedMessage {
            name: "Wrapper".to_string(),
            package: "test_pkg".to_string(),
            fields: vec![Field {
                name: "pose".to_string(),
                field_type: FieldType {
                    base_type: "Pose".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "Pose pose".to_string(),
            path: PathBuf::from("/tmp/test_pkg/msg/Wrapper.msg"),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver
            .resolve_messages(vec![point, pose, wrapper])
            .unwrap();
        let wrapper = resolved
            .into_iter()
            .find(|msg| msg.parsed.name == "Wrapper")
            .unwrap();

        assert_eq!(wrapper.definition.matches("MSG: test_pkg/Pose").count(), 1);
        assert_eq!(wrapper.definition.matches("MSG: test_pkg/Point").count(), 1);
    }

    #[test]
    fn test_resolve_simple_message() {
        let msg = ParsedMessage {
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
            source: "int32 value".to_string(),
            path: PathBuf::new(),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_messages(vec![msg]).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].parsed.name, "Simple");
        assert_eq!(resolved[0].schema_hash.0.len(), 32);
    }

    #[test]
    fn test_resolve_empty_message() {
        let msg = ParsedMessage {
            name: "Empty".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![],
            constants: vec![],
            source: "".to_string(),
            path: PathBuf::new(),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_messages(vec![msg]).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].parsed.name, "Empty");
    }

    #[test]
    fn test_resolve_with_builtin_dependency() {
        // First, create the Time message that Stamped depends on
        let time_msg = ParsedMessage {
            name: "Time".to_string(),
            package: "builtin_interfaces".to_string(),
            fields: vec![
                Field {
                    name: "sec".to_string(),
                    field_type: FieldType {
                        base_type: "int32".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
                Field {
                    name: "nanosec".to_string(),
                    field_type: FieldType {
                        base_type: "uint32".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
            ],
            constants: vec![],
            source: "int32 sec\nuint32 nanosec".to_string(),
            path: PathBuf::new(),
        };

        let msg = ParsedMessage {
            name: "Stamped".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![
                Field {
                    name: "timestamp".to_string(),
                    field_type: FieldType {
                        base_type: "Time".to_string(),
                        package: Some("builtin_interfaces".to_string()),
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
                Field {
                    name: "data".to_string(),
                    field_type: FieldType {
                        base_type: "int32".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
            ],
            constants: vec![],
            source: "builtin_interfaces/Time timestamp\nint32 data".to_string(),
            path: PathBuf::new(),
        };

        let mut resolver = Resolver::new();
        // Resolve both messages - Time first, then Stamped
        let resolved = resolver.resolve_messages(vec![time_msg, msg]).unwrap();

        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|r| r.parsed.name == "Stamped"));
    }

    #[test]
    fn test_resolve_nested_messages() {
        // Create Point message
        let point = ParsedMessage {
            name: "Point".to_string(),
            package: "geometry_msgs".to_string(),
            fields: vec![
                Field {
                    name: "x".to_string(),
                    field_type: FieldType {
                        base_type: "float64".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
                Field {
                    name: "y".to_string(),
                    field_type: FieldType {
                        base_type: "float64".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                },
            ],
            constants: vec![],
            source: "float64 x\nfloat64 y".to_string(),
            path: PathBuf::new(),
        };

        // Create Pose message that depends on Point
        let pose = ParsedMessage {
            name: "Pose".to_string(),
            package: "geometry_msgs".to_string(),
            fields: vec![Field {
                name: "position".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: Some("geometry_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "geometry_msgs/Point position".to_string(),
            path: PathBuf::new(),
        };

        let mut resolver = Resolver::new();
        let resolved = resolver.resolve_messages(vec![point, pose]).unwrap();

        assert_eq!(resolved.len(), 2);

        // Find the Pose message
        let pose_resolved = resolved.iter().find(|r| r.parsed.name == "Pose").unwrap();
        assert!(
            pose_resolved
                .definition
                .contains("MSG: geometry_msgs/Point")
        );
    }

    #[test]
    fn test_hash_deterministic() {
        let msg = ParsedMessage {
            name: "Test".to_string(),
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
            source: "int32 value".to_string(),
            path: PathBuf::new(),
        };

        let mut resolver1 = Resolver::new();
        let resolved1 = resolver1.resolve_messages(vec![msg.clone()]).unwrap();

        let mut resolver2 = Resolver::new();
        let resolved2 = resolver2.resolve_messages(vec![msg]).unwrap();

        assert_eq!(resolved1[0].schema_hash, resolved2[0].schema_hash);
    }

    #[test]
    fn resolve_action_uses_canonical_action_descriptor_instead_of_goal_hash() {
        let action = parse_test_action("Fibonacci");
        let mut resolver = Resolver::new();

        let resolved = resolver.resolve_action(action).unwrap();

        assert_ne!(resolved.schema_hash, resolved.goal.schema_hash);
    }

    #[test]
    fn resolve_action_protocol_hashes_use_canonical_descriptors_and_bundles() {
        let action = parse_test_action("Fibonacci");
        let mut resolver = Resolver::new();

        let resolved = resolver.resolve_action(action).unwrap();

        let send_goal = ros_z_schema::ServiceDef::new(
            "test_actions::FibonacciSendGoal",
            "test_actions::FibonacciSendGoalRequest",
            "test_actions::FibonacciSendGoalResponse",
        )
        .unwrap();
        assert_eq!(
            resolved.send_goal_hash,
            ros_z_schema::compute_hash(&send_goal)
        );

        let get_result = ros_z_schema::ServiceDef::new(
            "test_actions::FibonacciGetResult",
            "test_actions::FibonacciGetResultRequest",
            "test_actions::FibonacciGetResultResponse",
        )
        .unwrap();
        assert_eq!(
            resolved.get_result_hash,
            ros_z_schema::compute_hash(&get_result)
        );

        let feedback_message = add_bundle_definitions(
            add_bundle_definitions(
                SchemaBundle::builder("test_actions::FibonacciFeedbackMessage").definition(
                    "test_actions::FibonacciFeedbackMessage",
                    TypeDef::Struct(StructDef {
                        fields: vec![
                            FieldDef::new(
                                "goal_id",
                                FieldShape::Named(TypeName::new("ros_z::action::GoalId").unwrap()),
                            ),
                            FieldDef::new(
                                "feedback",
                                FieldShape::Named(
                                    TypeName::new("test_actions::FibonacciFeedback").unwrap(),
                                ),
                            ),
                        ],
                    }),
                ),
                &uuid_bundle(),
            ),
            &resolved.feedback.schema,
        )
        .build()
        .unwrap();
        assert_eq!(
            resolved.feedback_message_hash,
            ros_z_schema::compute_hash(&feedback_message)
        );

        let cancel_goal = ros_z_schema::ServiceDef::new(
            "ros_z::action::CancelGoal",
            "ros_z::action::CancelGoalRequest",
            "ros_z::action::CancelGoalResponse",
        )
        .unwrap();
        assert_eq!(
            resolved.cancel_goal_hash,
            ros_z_schema::compute_hash(&cancel_goal)
        );

        assert_eq!(
            resolved.status_hash,
            ros_z_schema::compute_hash(&goal_status_array_bundle())
        );
    }
}
