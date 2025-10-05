import json
from dataclasses import asdict, dataclass

import mujoco
from mujoco_rust_server import SimulationServer


@dataclass(frozen=True, kw_only=True)
class SceneDescription:
    meshes: dict
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
        server.register_scene(json.dumps(asdict(scene_description)))

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

            meshes[name] = {"vertices": verts, "faces": faces}

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
                        {
                            "name": geom_name,
                            "mesh": mesh_name,
                            "rgba": rgba,
                            "pos": self.model.geom_pos[g].tolist(),
                            "quat": self.model.geom_quat[g].tolist(),
                        }
                    )

            bodies[body_name] = {
                "id": i,
                "parent": mujoco.mj_id2name(
                    self.model, mujoco.mjtObj.mjOBJ_BODY.value, parent
                )
                if parent != -1
                else None,
                "geoms": geoms,
            }

        return SceneDescription(meshes=meshes, lights=lights, bodies=bodies)

    def publish(self, data: mujoco.MjData) -> None:
        state = {"timestamp": data.time, "bodies": {}}

        for i in range(self.model.nbody):
            name = mujoco.mj_id2name(
                self.model, mujoco.mjtObj.mjOBJ_BODY.value, i
            )
            pos = data.xpos[i].tolist()
            quat = data.xquat[i].tolist()  # (w, x, y, z)
            state["bodies"][name] = {"pos": pos, "quat": quat}

        self.server.update_scene_state(json.dumps(state))


# @app.get("/scene")
# async def get_scene():
#     return JSONResponse(content=export_scene(model))


# ========== 2. Dynamic State WebSocket ==========

# async def sim_loop(websocket: WebSocket):
#     await websocket.accept()
#     while True:
#         mujoco.mj_step(model, data)

#         state = {
#             "timestamp": data.time,
#             "bodies": {}
#         }

#         for i in range(model.nbody):
#             name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY, i)
#             pos = data.xpos[i].tolist()
#             quat = data.xquat[i].tolist()  # (w, x, y, z)
#             state["bodies"][name] = {"pos": pos, "quat": quat}

#         await websocket.send_text(json.dumps(state))
#         await asyncio.sleep(1.0 / 60.0)  # stream at ~60Hz


# @app.websocket("/state")
# async def websocket_endpoint(websocket: WebSocket):
#     await sim_loop(websocket)
# ```
