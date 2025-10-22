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


class SceneExporter:
    def __init__(
        self,
        *,
        server: SimulationServer,
        model: mujoco.MjModel,
    ) -> None:
        self.server = server
        self.model = model

        scene_description = self.export_scene()
        data = msgpack.packb(asdict(scene_description))
        server.register_scene(data)

    def export_scene(self) -> SceneDescription:
        """
        Extract static scene info:
        meshes, textures, lights, and body→geom mapping.
        """

        # Meshes
        meshes = {}
        for i in range(self.model.nmesh):
            name = mujoco.mj_id2name(
                self.model, mujoco.mjtObj.mjOBJ_MESH.value, i
            )
            vert_adr = self.model.mesh_vertadr[i]
            nvert = self.model.mesh_vertnum[i]
            face_adr = self.model.mesh_faceadr[i]
            nface = self.model.mesh_facenum[i]

            verts = self.model.mesh_vert[vert_adr : vert_adr + nvert].tolist()
            faces = self.model.mesh_face[face_adr : face_adr + nface].tolist()

            meshes[name] = Mesh(vertices=verts, faces=faces)

        # Textures (export raw for now)
        # textures = {}
        # for i in range(self.model.ntex):
        #     name = mujoco.mj_id2name(
        #         self.model, mujoco.mjtObj.mjOBJ_TEXTURE.value, i
        #     )
        #     width = self.model.tex_width[i]
        #     height = self.model.tex_height[i]
        #     address = self.model.tex_adr[i]
        #     tex_data = self.model.tex_data[
        #         address : address + width * height * 3
        #     ].tolist()
        #     textures[name] = {
        #         "width": width, "height": height, "rgb": tex_data
        #     }

        # Lights
        lights = []
        for i in range(self.model.nlight):
            name = mujoco.mj_id2name(
                self.model, mujoco.mjtObj.mjOBJ_LIGHT.value, i
            )
            position = self.model.light_pos[i].tolist()
            direction = self.model.light_dir[i].tolist()
            lights.append({"name": name, "pos": position, "dir": direction})

        # Bodies and attached geoms
        bodies = {}
        for i in range(self.model.nbody):
            body_name = mujoco.mj_id2name(
                self.model, mujoco.mjtObj.mjOBJ_BODY.value, i
            )
            parent = self.model.body_parentid[i]

            geoms = []
            for g in range(self.model.ngeom):
                if self.model.geom_bodyid[g] == i:
                    geom_name = mujoco.mj_id2name(
                        self.model, mujoco.mjtObj.mjOBJ_GEOM.value, g
                    )
                    mesh_name = None
                    if self.model.geom_type[g] == mujoco.mjtGeom.mjGEOM_MESH:
                        mesh_name = mujoco.mj_id2name(
                            self.model,
                            mujoco.mjtObj.mjOBJ_MESH.value,
                            self.model.geom_dataid[g],
                        )
                    rgba = self.model.geom_rgba[g].tolist()
                    geoms.append(
                        Geom(
                            name=geom_name,
                            mesh=mesh_name,
                            rgba=rgba,
                            pos=self.model.geom_pos[g].tolist(),
                            quat=self.model.geom_quat[g].tolist(),
                        )
                    )

            bodies[body_name] = Body(
                id=i,
                parent=mujoco.mj_id2name(
                    self.model, mujoco.mjtObj.mjOBJ_BODY.value, parent
                )
                if parent != -1
                else None,
                geoms=geoms,
            )

        return SceneDescription(meshes=meshes, lights=lights, bodies=bodies)
