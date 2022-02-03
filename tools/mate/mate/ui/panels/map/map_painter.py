import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import math


class Painter(qtg.QPainter):
    def __init__(self, pixels_per_meter: float):
        super(Painter, self).__init__()

        self.pixels_per_meter = pixels_per_meter

    def drawText(self, position: qtc.QPointF, text: str, font_size: float):
        """ Wrapper function to draw text in meter world.
        @param position QPointF: top left position of the text
        @param text to draw
        @param font_size in meter
        """
        self.save()
        transform_matrix = self.transform()
        self.resetTransform()
        self.setFont(qtg.QFont('Arial', self.pixels_per_meter * font_size))
        super(Painter, self).drawText(transform_matrix.map(position), text)
        self.restore()

    def drawRectF(self, x: float, y: float, w: float, h: float):
        """Provides floating point drawRect call."""
        self.drawRect(qtc.QRectF(x, y, w, h))

    def drawLineF(self, x1: float, y1: float, x2: float, y2: float):
        """Provides floating point drawLine call."""
        self.drawLine(qtc.QLineF(x1, y1, x2, y2))

    def drawPose(self, pose, diameter: float, line_length: float):
        """Draws the given pose as a circle with an orientation line."""
        self.drawEllipse(
            qtc.QPointF(pose[0][0], pose[0][1]),
            diameter / 2,
            diameter / 2
        )
        # line for orientation
        rot_vector = (math.cos(pose[1]), math.sin(pose[1]))
        self.drawLineF(
            pose[0][0],
            pose[0][1],
            pose[0][0] + line_length * rot_vector[0],
            pose[0][1] + line_length * rot_vector[1]
        )

    def drawTarget(self, target_pose, diameter):
        """Draws the given target pose as a cross hair"""
        self.drawEllipse(
            qtc.QPointF(target_pose[0][0], target_pose[0][1]),
            diameter / 2,
            diameter / 2
        )
        self.drawLineF(
            target_pose[0][0] - diameter / 2,
            target_pose[0][1],
            target_pose[0][0] + diameter / 2,
            target_pose[0][1]
        )
        self.drawLineF(
            target_pose[0][0],
            target_pose[0][1] - diameter / 2,
            target_pose[0][0],
            target_pose[0][1] + diameter / 2
        )

    def drawFOV(self, pose, headYaw, maxDistance, cameraOpeningAngle):
        """Draws a Field-Of-View triangle"""
        self.triangle(
            pose[0][0],
            pose[0][1],
            pose[0][0] + maxDistance * math.cos(
                pose[1] + headYaw + math.radians(
                    cameraOpeningAngle) / 2),
            pose[0][1] + maxDistance * math.sin(
                pose[1] + headYaw + math.radians(
                    cameraOpeningAngle) / 2),
            pose[0][0] + maxDistance * math.cos(
                pose[1] + headYaw - math.radians(
                    cameraOpeningAngle) / 2),
            pose[0][1] + maxDistance * math.sin(
                pose[1] + headYaw - math.radians(
                    cameraOpeningAngle) / 2)
        )

    def ideal_text_color(self, bg_color: qtg.QColor):
        """Returns a text color that is visible on a given background color"""
        threshold = 105
        background_delta = (bg_color.red() * 0.299) + \
            (bg_color.green() * 0.587) + (bg_color.blue() * 0.114)
        if 255 - background_delta < threshold:
            return "#000000"
        else:
            return "#ffffff"

    def setPen(self, arg1, width: float = None):
        """Set painter pen to given pen or color with the specified width."""
        if width is None:
            super(Painter, self).setPen(arg1)
        else:
            super(Painter, self).setPen(qtg.QPen(arg1, width))

    def drawPose(self, pose, diameter: float, line_length: float, annotation: str = None, font_size: float = 0.3, font_offset=None):
        """Draws the given pose as a circle with an orientation line."""
        if font_offset is None:
            font_offset = [diameter, diameter]

        self.drawEllipse(
            qtc.QPointF(pose[0][0], pose[0][1]),
            diameter / 2,
            diameter / 2
        )
        # line for orientation
        rot_vector = (math.cos(pose[1]), math.sin(pose[1]))
        self.drawLineF(
            pose[0][0],
            pose[0][1],
            pose[0][0] + line_length * rot_vector[0],
            pose[0][1] + line_length * rot_vector[1]
        )
        if annotation:
            text_position = qtc.QPointF(
                pose[0][0] + font_offset[0], pose[0][1] + font_offset[1])
            self.drawText(text_position, annotation, font_size)

    def triangle(
            self,
            x1: float,
            y1: float,
            x2: float,
            y2: float,
            x3: float,
            y3: float):
        """Draws a triangle between the three provided points."""
        self.drawLineF(x1, y1, x2, y2)
        self.drawLineF(x2, y2, x3, y3)
        self.drawLineF(x3, y3, x1, y1)

    def transformByPose(self, pose):
        """transform coordinate-system to be relative to pose"""
        self.translate(pose[0][0], pose[0][1])
        self.rotate(math.degrees(pose[1]))

    def getPoseFromVector3(self, vector3):
        """convert vector3 to pose"""
        return [[vector3[0], vector3[1]], vector3[2]]
