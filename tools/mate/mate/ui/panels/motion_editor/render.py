import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import OpenGL.GL as gl
import OpenGL.GLU as glu
import math

from .motion_editor import *
from .nao_rig import NaoRig
from .joint import Joint


class RenderView(qtw.QOpenGLWidget):
    def __init__(self, model, parent):
        super(RenderView, self).__init__()
        self.rig = NaoRig()
        self.model = model
        self.camera = [0, 0, 1]  # [yaw, pitch, zoom]
        self.colorcounter = 0
        self.parent = parent

    def wheelEvent(self, event):
        if event.modifiers() == qtc.Qt.ControlModifier:  # zoom
            f = 1
            for x in range(abs(event.angleDelta().y() // 10)):
                f *= 1.01 if event.angleDelta().y() >= 0 else 0.99
            self.camera[2] *= f
        elif event.modifiers() == qtc.Qt.ShiftModifier:
            if self.model["selected_joint"] is not None and not self.model["live_mode"]:
                get_current_position(self.model)["parameters"][
                    self.model["highlight_joint_plot"]] -= event.angleDelta().y(
                    ) / 2400
                self.parent.update_highlighted_joint_table_item()
            else:
                return
        else:
            self.camera[1] -= event.angleDelta().x() / 60
            self.camera[0] -= event.angleDelta().y() / 60
            # prevent camera from flipping over
            self.camera[0] = max(min(self.camera[0], 90), -90)
        self.update()

    def renderGrid(self):
        gl.glColor3f(0.3, 0.3, 0.3)
        gl.glLineWidth(1)
        gl.glBegin(gl.GL_LINES)
        c3 = self.model["gridDistance"]
        for c1 in range(-c3, c3, c3 // 10):
            # back grid
            gl.glVertex3f(-c1, -c3, -c3)
            gl.glVertex3f(-c1, c3, -c3)
            gl.glVertex3f(-c3, -c1, -c3)
            gl.glVertex3f(c3, -c1, -c3)

            # left grid
            gl.glVertex3f(-c3, -c3, -c1)
            gl.glVertex3f(-c3, c3, -c1)
            gl.glVertex3f(-c3, -c1, -c3)
            gl.glVertex3f(-c3, -c1, c3)

            # bottom grid
            gl.glVertex3f(-c1, -c3, -c3)
            gl.glVertex3f(-c1, -c3, c3)
            gl.glVertex3f(-c3, -c3, -c1)
            gl.glVertex3f(c3, -c3, -c1)
        gl.glEnd()

    def renderPlot(self):
        scalar = float(self.size().width()) / float(self.size().height())
        c3 = 500
        hor_gap = 11
        from_top = 0.65

        gl.glEnable(gl.GL_BLEND)
        gl.glBlendFunc(gl.GL_SRC_ALPHA, gl.GL_ONE_MINUS_SRC_ALPHA) # use transparency blending for background

        # background
        gl.glColor4f(0.05, 0.05, 0.05, 0.75)
        gl.glLineWidth(1)
        gl.glBegin(gl.GL_TRIANGLE_STRIP)
        gl.glVertex3f(-scalar*c3 + hor_gap, -c3 * from_top, -c3)
        gl.glVertex3f(-scalar*c3 + hor_gap, -c3 + hor_gap, -c3)
        gl.glVertex3f(scalar*c3 - hor_gap, -c3 * from_top, -c3)
        gl.glVertex3f(scalar*c3 - hor_gap, -c3 + hor_gap, -c3)
        gl.glEnd()

        # t indicator
        gl.glColor3f(1.0, 0.3, 0.3)
        gl.glLineWidth(1)
        gl.glBegin(gl.GL_LINES)
        c3 -= 50 # draw everything else in front of the background
        t = float(self.model["t_to_reach_duration"]) / float(self.model["motion2_data"]["header"]["time"])

        def on_timeline_at(tx):
            return -scalar * c3 + hor_gap + (tx * (scalar * c3 * 2 - (2 * hor_gap)))

        gl.glVertex3f(on_timeline_at(t), -c3 + hor_gap, -c3)
        gl.glVertex3f(on_timeline_at(t), -c3 * from_top, -c3)
        gl.glEnd()

        # ticks
        time_accumulator = 0
        for frame_index in range(len(self.model["motion2_data"]["position"])):
            frame_begin = float(time_accumulator) / float(self.model["motion2_data"]["header"]["time"])
            time_accumulator += self.model["motion2_data"]["position"][frame_index]["time"]
            frame_end = float(time_accumulator) / float(self.model["motion2_data"]["header"]["time"])

            if frame_index == self.model["current_frame"]:
                gl.glColor3f(0.2, 0.2, 0.2)
                gl.glLineWidth(1)
                gl.glBegin(gl.GL_TRIANGLE_STRIP)
                gl.glVertex3f(on_timeline_at(frame_begin), -c3 + hor_gap, -c3)
                gl.glVertex3f(on_timeline_at(frame_begin), -c3 * from_top, -c3)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 + hor_gap, -c3)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 * from_top, -c3)
                gl.glEnd()
                gl.glColor3f(1.0, 1.0, 1.0)
                gl.glLineWidth(3)
                gl.glBegin(gl.GL_LINES)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 + hor_gap, -c3)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 * from_top, -c3)
                gl.glEnd()
            else:
                gl.glColor3f(0.4, 0.4, 0.4)
                gl.glLineWidth(1)
                gl.glBegin(gl.GL_LINES)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 + hor_gap, -c3)
                gl.glVertex3f(on_timeline_at(frame_end), -c3 * from_top, -c3)
                gl.glEnd()

        # curves
        num_of_joints = len(self.model["motion2_data"]["position"][0]["parameters"])
        num_of_frames = len(self.model["motion2_data"]["position"])

        height_scalar = 30
        width_scalar = (scalar * c3) - hor_gap
        height_offset = -185

        gl.glBlendFunc(gl.GL_SRC_ALPHA, gl.GL_ONE) # use additive blending for graph
        for angle_index in range(num_of_joints):
            joint_index = angle_index #get_joint_index(self.model, angle_index)
            gl.glColor3f(0.3, 0.3, 0.75)
            gl.glLineWidth(1)
            accumulated_time = self.model["motion2_data"]["position"][0]["time"]

            for frame_index in range(num_of_frames):
                if frame_index > 0:
                    color_scalar = abs(float(self.model["motion2_data"]["position"][frame_index]["parameters"][joint_index]) - float(self.model["motion2_data"]["position"][frame_index-1]["parameters"][joint_index])) / float(self.model["motion2_data"]["position"][frame_index]["time"]) * 1000.0
                    color_scalar = min(1.0, color_scalar)
                    if joint_index == self.model["highlight_joint_plot"]:
                        color_scalar = 1.0
                        if frame_index == self.model["current_frame"] or frame_index - 1 == self.model["current_frame"]:
                            gl.glColor3f(0.298, 0.71, 0.922)
                        else:
                            gl.glColor3f(1.0, 1.0, 1.0)
                        gl.glLineWidth(3)
                    else:
                        gl.glColor4f(0.7, 0.7, 1.0, color_scalar)
                        gl.glLineWidth(1)
                    if (color_scalar > 0.01):
                        gl.glBegin(gl.GL_LINES)
                        gl.glVertex3f(
                            -scalar * c3 * 0.5 + (hor_gap / 2) + ((accumulated_time / float(self.model["motion2_data"]["header"]["time"])) * width_scalar),
                            self.model["motion2_data"]["position"][frame_index-1]["parameters"][joint_index] * height_scalar * 0.5 + height_offset,
                            -c3 * 0.5)
                    accumulated_time += self.model["motion2_data"]["position"][frame_index]["time"]
                    if (color_scalar > 0.01):
                        gl.glVertex3f(
                            -scalar * c3 * 0.5 + (hor_gap / 2) + ((accumulated_time / float(self.model["motion2_data"]["header"]["time"])) * width_scalar),
                            self.model["motion2_data"]["position"][frame_index]["parameters"][joint_index] * height_scalar * 0.5 + height_offset,
                            -c3 * 0.5)
                        gl.glEnd()

    def drawArrow(self, joint, rotation):
        gl.glPushMatrix()

        gl.glColor3f(0, 1, 0)
        yshift = joint.length/2
        if rotation[0] == 0:
            yshift = 0
            self.rotate(90, 2)
        if rotation[0] == 2:
            yshift = 0
            self.rotate(90, 0)

        if len(rotation) > 2:
            multiplier = rotation[2]
        else:
            multiplier = 1

        gl.glBegin(gl.GL_TRIANGLE_STRIP)
        diameter = 15
        width = 2
        a = 0
        for i in range(28):
            a = math.radians(i * -10 * multiplier)
            gl.glVertex3f(math.cos(a)*diameter, yshift + width/2, math.sin(a)*diameter)
            gl.glVertex3f(math.cos(a)*diameter, yshift - width/2, math.sin(a)*diameter)
        gl.glEnd()
        gl.glBegin(gl.GL_TRIANGLES)
        gl.glVertex3f(math.cos(a)*diameter, yshift+width, math.sin(a)*diameter)
        gl.glVertex3f(math.cos(a)*diameter, yshift-width, math.sin(a)*diameter)
        a += math.radians(-30 * multiplier)
        gl.glVertex3f(math.cos(a)*diameter, yshift, math.sin(a)*diameter)
        gl.glEnd()

        gl.glPopMatrix()

    def drawModel(self, joint: Joint):
        r = 1 - (self.colorcounter // 9 % 3) * 0.5
        g = 1 - (self.colorcounter // 3 % 3) * 0.5
        b = 1 - (self.colorcounter % 3) * 0.5
        self.colorcounter += 1
        gl.glColor3f(r, g, b)

        # special handling for selected joints
        gl.glLineWidth(1)
        for rotation in joint.pose_angles:
            if rotation[1] == self.model["selected_joint"]:
                self.drawArrow(joint, rotation)
                gl.glLineWidth(3)
                gl.glColor3f(1, 1, 1)

        gl.glBegin(gl.GL_LINE_LOOP)
        d = joint.length
        w = self.model["boneWidth"]
        gl.glVertex3f(0, 0, 0)
        gl.glVertex3f(w, d * self.model["bulkPosition"], w)
        gl.glVertex3f(0, d, 0)
        gl.glVertex3f(-w, d * self.model["bulkPosition"], w)
        gl.glVertex3f(0, 0, 0)
        gl.glVertex3f(w, d * self.model["bulkPosition"], -w)
        gl.glVertex3f(0, d, 0)
        gl.glVertex3f(-w, d * self.model["bulkPosition"], -w)
        gl.glVertex3f(0, 0, 0)
        gl.glEnd()

    def rotate(self, angle, axis_index):
        """
        Rotate the modelview matrix around a given axis

        :param angle: angle of rotation in degrees
        :param axis_index: index of the base axis. x=0, y=1, z=2
        """
        gl.glRotatef(angle, 1 if axis_index == 0 else 0, 1
                     if axis_index == 1 else 0, 1 if axis_index == 2 else 0)

    def renderJoint(self, joint: Joint, angles):
        gl.glPushMatrix()

        for axis in [0, 1, 2]:
            self.rotate(joint.base_angles[axis], axis)

        for rotation in joint.pose_angles:
            axis, angle_index = rotation[0], rotation[1]
            if len(rotation) > 2:
                multiplier = rotation[2]
            else:
                multiplier = 1
            self.rotate(multiplier * math.degrees(
                angles[get_joint_index(self.model, angle_index)]), axis)

        self.drawModel(joint)

        gl.glTranslatef(0, joint.length, 0)
        for child in joint.children:
            self.renderJoint(child, angles)

        gl.glPopMatrix()

    def renderRig(self, angles):
        self.colorcounter = 0
        self.renderJoint(self.rig.body, angles)

    def paintGL(self):
        gl.glClearColor(0, 0, 0, 1)  # background color (RGBA)
        gl.glClear(gl.GL_COLOR_BUFFER_BIT | gl.GL_DEPTH_BUFFER_BIT)

        if not self.model["valid"] and not self.model["live_mode"]:
            return

        # setup camera
        gl.glLoadIdentity()
        gl.glTranslatef(0, 0, -500)
        gl.glRotatef(self.camera[0], 1, 0, 0)
        gl.glRotatef(self.camera[1], 0, 1, 0)
        gl.glScalef(self.camera[2], self.camera[2], self.camera[2])

        if self.model["showGrid"]:
            self.renderGrid()

        if self.model["live_mode"]:
            self.renderRig(self.model["live_angles"])
        else:
            if self.model["showRig"]:
                self.renderRig(get_interpolated_position(self.model))

        if self.model["showPlot"]:
            gl.glClear(gl.GL_DEPTH_BUFFER_BIT) # prevent scene - HUD clipping
            gl.glLoadIdentity()
            self.renderPlot()

        gl.glFinish()  # swap buffers

    def resizeGL(self, width, height):
        gl.glViewport(0, 0, width, height)

        gl.glMatrixMode(gl.GL_PROJECTION)
        gl.glLoadIdentity()
        glu.gluPerspective(90, width / height, 0.1, 10000)
        gl.glMatrixMode(gl.GL_MODELVIEW)

    def initializeGL(self):
        gl.glEnable(gl.GL_DEPTH_TEST)
