use factrs::{
    containers::FactorBuilder,
    core::{GaussNewton, Graph, PriorResidual, Values},
    noise::GaussianNoise,
    optimizers::BaseOptParams,
    traits::Optimizer,
};

use crate::{
    factors::field_containment::FieldContainmentFactor,
    initial_state::InitialState,
    symbols::{CameraIntrinsics, State},
};

use super::{
    BackendConfiguration, INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA, INITIAL_POSE_PRIOR_SIGMA,
};

pub(crate) fn initialize_graph(
    initial_state: &InitialState,
    config: &BackendConfiguration,
) -> (Graph, Values) {
    let mut graph = Graph::default();
    let mut values = Values::default();

    let factor = FactorBuilder::new(
        PriorResidual::new(initial_state.camera_intrinsics.clone()),
        CameraIntrinsics(0),
    )
    .noise(GaussianNoise::<4>::from_diag_sigmas(
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
    ))
    .build();
    graph.add_factor(factor);
    values.insert(CameraIntrinsics(0), initial_state.camera_intrinsics.clone());

    let initial_pose = initial_state.pose.clone();
    let factor = FactorBuilder::new(PriorResidual::new(initial_pose.clone()), State(0))
        .noise(GaussianNoise::<9>::from_scalar_sigma(
            INITIAL_POSE_PRIOR_SIGMA,
        ))
        .build();
    graph.add_factor(factor);
    values.insert(State(0), initial_pose);
    add_field_containment_factor(&mut graph, State(0), config);

    (graph, values)
}

pub(super) fn optimizer_from_graph(config: &BackendConfiguration, graph: Graph) -> GaussNewton {
    let mut optimizer = GaussNewton::new(
        BaseOptParams {
            max_iterations: config.optimizer_max_iterations,
            ..Default::default()
        },
        graph,
    );
    optimizer.set_dense_normal_equations(true);
    optimizer
}

pub(super) fn add_field_containment_factor(
    graph: &mut Graph,
    state: State,
    config: &BackendConfiguration,
) {
    let containment = config.field_containment;
    let residual =
        FieldContainmentFactor::new(containment.x_limit, containment.y_limit, containment.sigma);
    graph.add_factor(FactorBuilder::new(residual, state).build());
}
