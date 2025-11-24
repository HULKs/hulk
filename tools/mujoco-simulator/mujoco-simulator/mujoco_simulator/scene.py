import json
from dataclasses import asdict, dataclass

import msgpack
import mujoco
from mujoco import MjModel
from mujoco._structs import MjData

from mujoco_rust_server import (Body, BodyUpdate, Geom, Light, SceneDescription, SceneMesh, SceneUpdate)


def generate_scene_description(model: MjModel) -> SceneDescription:
    # Meshes
    meshes = {}
    for i in range(model.nmesh):
        name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_MESH.value, i)
        vert_adr = model.mesh_vertadr[i]
        nvert = model.mesh_vertnum[i]
        face_adr = model.mesh_faceadr[i]
        nface = model.mesh_facenum[i]

        verts = model.mesh_vert[vert_adr : vert_adr + nvert].tolist()
        faces = model.mesh_face[face_adr : face_adr + nface].tolist()

        meshes[name] = SceneMesh(vertices=verts, faces=faces)

    # Textures (export raw for now)
    # textures = {}
    # for i in range(model.ntex):
    #     name = mujoco.mj_id2name(
    #         model, mujoco.mjtObj.mjOBJ_TEXTURE.value, i
    #     )
    #     width = model.tex_width[i]
    #     height = model.tex_height[i]
    #     address = model.tex_adr[i]
    #     tex_data = model.tex_data[
    #         address : address + width * height * 3
    #     ].tolist()
    #     textures[name] = {
    #         "width": width, "height": height, "rgb": tex_data
    #     }

    # Lights
    lights = []
    for i in range(model.nlight):
        name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_LIGHT.value, i)
        position = model.light_pos[i].tolist()
        direction = model.light_dir[i].tolist()
        lights.append(Light(name, position, direction))

    # Bodies and attached geoms
    bodies = {}
    for i in range(model.nbody):
        body_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY.value, i)
        assert body_name is not None, f"Body name is None for body id {i}"
        parent = model.body_parentid[i]

        geoms = []
        for g in range(model.ngeom):
            if model.geom_bodyid[g] == i:
                geom_name = mujoco.mj_id2name(
                    model, mujoco.mjtObj.mjOBJ_GEOM.value, g
                )
                mesh_name = None
                if model.geom_type[g] == mujoco.mjtGeom.mjGEOM_MESH:
                    mesh_name = mujoco.mj_id2name(
                        model,
                        mujoco.mjtObj.mjOBJ_MESH.value,
                        model.geom_dataid[g],
                    )
                rgba = model.geom_rgba[g].tolist()
                geoms.append(
                    Geom(
                        name=geom_name,
                        mesh=mesh_name,
                        rgba=rgba,
                        pos=model.geom_pos[g].tolist(),
                        quat=model.geom_quat[g].tolist(),
                    )
                )

        bodies[body_name] = Body(
            id=i,
            parent=mujoco.mj_id2name(
                model, mujoco.mjtObj.mjOBJ_BODY.value, parent
            )
            if parent != -1
            else None,
            geoms=geoms,
        )

    return SceneDescription(meshes=meshes, lights=lights, bodies=bodies)


def generate_scene_state(model: MjModel, data: MjData) -> SceneUpdate:
    bodies = {}

    for i in range(model.nbody):
        name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY.value, i)
        pos = data.xpos[i].tolist()
        quat = data.xquat[i].tolist()  # (w, x, y, z)
        bodies[name] = BodyUpdate(pos=pos, quat=quat)

    return SceneUpdate(time=data.time, bodies=bodies)

