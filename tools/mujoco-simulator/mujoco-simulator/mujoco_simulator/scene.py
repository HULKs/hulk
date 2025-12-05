import logging

import mujoco
import numpy as np
from mujoco import MjModel
from mujoco._structs import MjData
from mujoco_rust_server import (
    Body,
    BodyUpdate,
    Geom,
    Light,
    Material,
    PbrMaterial,
    SceneDescription,
    SceneMesh,
    SceneUpdate,
    Texture,
)


def material_for_geom(model: MjModel, geom_index: int) -> Material:
    material_index = model.geom_matid[geom_index]
    if material_index < 0:
        rgba: list[float] = model.geom_rgba[geom_index].tolist()
        return Material.rgba(rgba)
    return Material.pbr(material_index)


def resolve_geom(model: MjModel, geom_index: int) -> Geom | None:
    geom_type = mujoco.mjtGeom(model.geom_type[geom_index])
    pos: list[float] = model.geom_pos[geom_index].tolist()
    quat: list[float] = model.geom_quat[geom_index].tolist()
    material = material_for_geom(model, geom_index)

    if geom_type == mujoco.mjtGeom.mjGEOM_MESH:
        return Geom.mesh(
            index=geom_index,
            mesh_index=model.geom_dataid[geom_index],
            material=material,
            pos=pos,
            quat=quat,
        )

    if geom_type == mujoco.mjtGeom.mjGEOM_SPHERE:
        radius: float = model.geom_size[geom_index][0]
        return Geom.sphere(
            index=geom_index,
            radius=radius,
            material=material,
            pos=pos,
            quat=quat,
        )

    if geom_type == mujoco.mjtGeom.mjGEOM_BOX:
        extent: list[float] = model.geom_size[geom_index].tolist()
        return Geom.box(
            index=geom_index,
            extent=extent,
            material=material,
            pos=pos,
            quat=quat,
        )

    if geom_type == mujoco.mjtGeom.mjGEOM_PLANE:
        normal: list[float] = model.geom_size[geom_index].tolist()
        return Geom.plane(
            index=geom_index,
            normal=normal,
            material=material,
            pos=pos,
            quat=quat,
        )

    if geom_type == mujoco.mjtGeom.mjGEOM_CYLINDER:
        radius: float = model.geom_size[geom_index][0]
        half_height: float = model.geom_size[geom_index][1]
        return Geom.cylinder(
            index=geom_index,
            radius=radius,
            half_height=half_height,
            material=material,
            pos=pos,
            quat=quat,
        )

    logging.warning("Unhandled mujoco geom type:", geom_type)

    return None


def generate_scene_description(model: MjModel) -> SceneDescription:
    # Meshes
    meshes = {}
    for i in range(model.nmesh):

        def get_values(i: int, field: str, suffix: str="") -> list:
            start = getattr(model, f"mesh_{field}adr")[i]
            count = getattr(model, f"mesh_{field}num")[i]
            data = getattr(model, f"mesh_{field}{suffix}")
            return data[start : start + count].tolist()

        vertices = get_values(i, "vert")
        vertex_indices = get_values(i, "face")
        uv_coordinates = get_values(i, "texcoord")
        uv_indices = get_values(i, "face", "texcoord")

        meshes[i] = SceneMesh(
            vertices=vertices,
            vertex_indices=vertex_indices,
            uv_coordinates=uv_coordinates,
            uv_indices=uv_indices,
        )

    # Materials
    materials = {}
    for i in range(model.nmat):
        rgba = model.mat_rgba[i]
        textures = model.mat_texid[i]
        reflectance = model.mat_reflectance[i]
        shininess = model.mat_shininess[i]
        specular = model.mat_specular[i]

        textures = [texture if texture >= 0 else None for texture in textures]

        materials[i] = PbrMaterial(
            rgba, textures, reflectance, shininess, specular
        )

    # Textures (export raw for now)
    textures = {}
    for i in range(model.ntex):
        name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_TEXTURE.value, i)
        width = model.tex_width[i]
        height = model.tex_height[i]
        address = model.tex_adr[i]
        tex_data = model.tex_data[
            address : address + width * height * 3
        ].tolist()
        textures[i] = Texture(
            name=name,
            width=width,
            height=height,
            rgb=tex_data,
        )

    # Lights
    lights = []
    for i in range(model.nlight):
        name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_LIGHT.value, i)
        position = model.light_pos[i].tolist()
        direction = model.light_dir[i].tolist()
        lights.append(Light(name, position, direction))

    geoms = {}
    for i in range(model.ngeom):
        if geom := resolve_geom(model, i):
            geoms[i] = geom

    # Bodies and attached geoms
    bodies = {}
    for i in range(model.nbody):
        body_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY.value, i)
        parent = model.body_parentid[i]
        geom_indices = np.where(model.geom_bodyid == i)[0].tolist()

        bodies[i] = Body(
            id=i,
            name=body_name,
            parent=parent if parent != -1 else None,
            geoms=geom_indices,
        )

    return SceneDescription(
        meshes=meshes,
        materials=materials,
        lights=lights,
        bodies=bodies,
        geoms=geoms,
        textures=textures,
    )


def generate_scene_state(model: MjModel, data: MjData) -> SceneUpdate:
    bodies = {}

    for i in range(model.nbody):
        pos = data.xpos[i].tolist()
        quat = data.xquat[i].tolist()  # (w, x, y, z)
        bodies[i] = BodyUpdate(pos=pos, quat=quat)

    return SceneUpdate(time=data.time, bodies=bodies)
