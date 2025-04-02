from dataclasses import dataclass

import mujoco


@dataclass
class VisualizationState:
    show_contacts: bool = False
    show_joints: bool = False
    show_figures: bool = True
    transparent: bool = False
    show_shadows: bool = True
    show_wire_frame: bool = False
    show_convex_hull: bool = False
    show_inertias: bool = False
    show_com: bool = False
    show_overlay: bool = True
    show_frame: int = 0

    def toggle_contacts(self, option: mujoco.MjvOption) -> None:
        self.show_contacts = not self.show_contacts
        option.flags[mujoco.mjtVisFlag.mjVIS_CONTACTPOINT] = self.show_contacts
        option.flags[mujoco.mjtVisFlag.mjVIS_CONTACTFORCE] = self.show_contacts

    def toggle_joints(self, option: mujoco.MjvOption) -> None:
        self.show_joints = not self.show_joints
        option.flags[mujoco.mjtVisFlag.mjVIS_JOINT] = self.show_joints

    def cycle_frame_display(self, option: mujoco.MjvOption) -> None:
        self.show_frame += 1
        if self.show_frame == mujoco.mjtFrame.mjNFRAME.value:
            self.show_frame = 0
        option.frame = self.show_frame

    def toggle_overlays(self) -> None:
        self.show_overlay = not self.show_overlay

    def toggle_transparency(self, model: mujoco.MjModel) -> None:
        self.transparent = not self.transparent
        if self.transparent:
            model.geom_rgba[:, 3] /= 5.0
        else:
            model.geom_rgba[:, 3] *= 5.0

    def toggle_figures(self) -> None:
        self.show_figures = not self.show_figures

    def toggle_inertias(self, option: mujoco.MjvOption) -> None:
        self.show_inertias = not self.show_inertias
        option.flags[mujoco.mjtVisFlag.mjVIS_INERTIA] = self.show_inertias

    def toggle_com(self, option: mujoco.MjvOption) -> None:
        self.show_com = not self.show_com
        option.flags[mujoco.mjtVisFlag.mjVIS_COM] = self.show_com

    def toggle_shadows(self, scene: mujoco.MjvScene) -> None:
        self.show_shadows = not self.show_shadows
        scene.flags[mujoco.mjtRndFlag.mjRND_SHADOW] = self.show_shadows

    def toggle_convex_hull(self, option: mujoco.MjvOption) -> None:
        self.show_convex_hull = not self.show_convex_hull
        option.flags[mujoco.mjtVisFlag.mjVIS_CONVEXHULL] = self.show_convex_hull

    def toggle_wire_frame(self, scene: mujoco.MjvScene) -> None:
        self.show_wire_frame = not self.show_wire_frame
        scene.flags[mujoco.mjtRndFlag.mjRND_WIREFRAME] = self.show_wire_frame
