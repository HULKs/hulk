import json
from dataclasses import asdict, dataclass

import msgpack
import mujoco
from mujoco_rust_server import SimulationServer


@dataclass(frozen=True, kw_only=True)
class Mesh:
    vertices: list[list[float]]
    faces: list[list[int]]


@dataclass(frozen=True, kw_only=True)
class Geom:
    name: str
    mesh: str | None
    rgba: list[float]
    pos: list[float]
    quat: list[float]


@dataclass(frozen=True, kw_only=True)
class Body:
    id: int
    parent: str | None
    geoms: list[Geom]


@dataclass(frozen=True, kw_only=True)
class SceneDescription:
    meshes: dict[str, Mesh]
    # textures: dict
    lights: list
    bodies: dict


def export_scene(model: mujoco.MjModel) -> SceneDescription:
    """
    Extract static scene info:
    meshes, textures, lights, and body→geom mapping.
    """

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

        meshes[name] = Mesh(vertices=verts, faces=faces)

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
        lights.append({"name": name, "pos": position, "dir": direction})

    # Bodies and attached geoms
    bodies = {}
    for i in range(model.nbody):
        body_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY.value, i)
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


def serialize(scene_description: SceneDescription) -> bytes:
    return msgpack.packb(asdict(scene_description))


class SceneExporter:
    def __init__(
        self,
        *,
        server: SimulationServer,
        model: mujoco.MjModel,
    ) -> None:
        self.server = server

        scene_description = export_scene(model)
        data = serialize(scene_description)
        server.register_scene(data)
