import os
import re
import sys
import tempfile
import time
from dataclasses import dataclass
from enum import Enum

import cv2
import numpy as np
from numpy.typing import NDArray
from PyQt6.QtWidgets import QApplication, QFileDialog
from src.decision_tree import optimize_thresholds

IMAGE_DIRECTORY = "/home/franziska-sophie/Downloads/tmp"  # os.path.join(tempfile.gettempdir(), "twix")

colors = Enum(
    "colors",
    [
        ("FIELD_COLOR", (0, 255, 0)),
        ("NOT_FIELD_COLOR", (255, 0, 0)),
        ("DELETE_COLOR", (0, 0, 0)),
        ("WHITE", (255, 255, 255)),
    ],
)


@dataclass
class DrawingBoard:
    ix: int = -1
    iy: int = -1
    brush_size: int = 11
    cursor_position: tuple[int, int] = (0, 0)
    drawing: bool = False
    color: tuple[int, int, int] = colors.FIELD_COLOR
    starting_position_line: tuple[int, int] = (0, 0)
    line: bool = False


def draw_with_drag(
    event: int, x: int, y: int, flags: any, param: DrawingBoard
) -> None:
    _ = flags
    drawing_board = param
    drawing_board.cursor_position = (x, y)
    brush_size = drawing_board.brush_size
    color = drawing_board.color

    if event == cv2.EVENT_LBUTTONDOWN:
        if drawing_board.line:
            drawing_board.starting_position_line = (x, y)
            cv2.circle(
                overlay,
                drawing_board.starting_position_line,
                int(brush_size / 2),
                color.value,
                -1,
            )
        drawing_board.drawing = True
        drawing_board.ix = x
        drawing_board.iy = y
    elif event == cv2.EVENT_MOUSEMOVE:
        if drawing_board.drawing and not drawing_board.line:
            cv2.circle(overlay, (x, y), brush_size, color.value, -1)
    elif event == cv2.EVENT_LBUTTONUP:
        drawing_board.drawing = False
        if drawing_board.line:
            cv2.line(
                overlay,
                drawing_board.starting_position_line,
                (x, y),
                color.value,
                brush_size,
            )


def extract_pixels(
    overlay: NDArray[np.integer],
    pixels_YCrCb: NDArray[np.integer],
    pixels_BGR: NDArray[np.integer],
    image_YCrCb: NDArray[np.integer],
    image_BGR: NDArray[np.integer],
    y: NDArray[np.integer],
) -> tuple[NDArray, NDArray, NDArray]:
    not_field_mask = np.all(overlay == colors.NOT_FIELD_COLOR.value, axis=-1)
    field_mask = np.all(overlay == colors.FIELD_COLOR.value, axis=-1)
    pixels_YCrCb = np.append(pixels_YCrCb, image_YCrCb[not_field_mask], axis=0)
    pixels_YCrCb = np.append(pixels_YCrCb, image_YCrCb[field_mask], axis=0)
    pixels_BGR = np.append(pixels_BGR, image_BGR[not_field_mask], axis=0)
    pixels_BGR = np.append(pixels_BGR, image_BGR[field_mask], axis=0)
    y = np.append(y, np.zeros(np.sum(not_field_mask)))
    y = np.append(y, np.ones(np.sum(field_mask)))

    return pixels_YCrCb, pixels_BGR, y


def switch_coloring_mode(actual_color: colors, wanted_color: colors) -> colors:
    if actual_color == wanted_color:
        return colors.FIELD_COLOR
    return wanted_color


if __name__ == "__main__":
    app = QApplication(sys.argv)
    files, _ = QFileDialog.getOpenFileNames(
        None, "Select your files", IMAGE_DIRECTORY
    )

    pixels_YCrCb_top = np.empty((0, 3), dtype=np.uint8)
    pixels_BGR_top = np.empty((0, 3), dtype=np.uint8)
    y_top = np.empty(0)

    pixels_YCrCb_bottom = np.empty((0, 3), dtype=np.uint8)
    pixels_BGR_bottom = np.empty((0, 3), dtype=np.uint8)
    y_bottom = np.empty(0)

    labeled_images_folder = os.path.join(IMAGE_DIRECTORY, "labeled")
    os.makedirs(labeled_images_folder, exist_ok=True)

    for file_path in files:
        drawing_board = DrawingBoard()
        file_name = os.path.basename(file_path)
        image_CrCbY = cv2.imread(file_path)
        image_YCrCb = image_CrCbY[..., [2, 0, 1]]
        image_BGR = cv2.cvtColor(image_YCrCb, cv2.COLOR_YCrCb2BGR)

        overlay = cv2.imread(
            os.path.join(labeled_images_folder, f"{file_name[:-4]}_labeled.png")
        )

        top = re.search("top", file_name.lower())
        bottom = re.search("bottom", file_name.lower())

        if overlay is not None:
            if top is None and bottom is not None:
                pixels_YCrCb_bottom, pixels_BGR_bottom, y_bottom = (
                    extract_pixels(
                        overlay,
                        pixels_YCrCb_bottom,
                        pixels_BGR_bottom,
                        image_YCrCb,
                        image_BGR,
                        y_bottom,
                    )
                )
            elif top is not None and bottom is None:
                pixels_YCrCb_top, pixels_BGR_top, y_top = extract_pixels(
                    overlay,
                    pixels_YCrCb_top,
                    pixels_BGR_top,
                    image_YCrCb,
                    image_BGR,
                    y_top,
                )
        else:
            overlay = np.zeros_like(image_BGR, dtype=np.uint8)
            canvas = np.zeros_like(image_BGR, dtype=np.uint8)

            cv2.namedWindow("Label the image!")
            cv2.setMouseCallback(
                "Label the image!", draw_with_drag, param=drawing_board
            )

            while True:
                combined = image_BGR.copy()
                temp_canvas = canvas.copy()
                color = drawing_board.color

                cv2.circle(
                    temp_canvas,
                    drawing_board.cursor_position,
                    drawing_board.brush_size,
                    colors.WHITE.value,
                    1,
                )

                cv2.addWeighted(overlay, 0.5, combined, 1, 0, combined)
                cv2.addWeighted(temp_canvas, 1, combined, 1, 0, temp_canvas)
                cv2.imshow("Label the image!", temp_canvas)

                key = cv2.waitKey(50) & 0xFF

                if key == ord("q"):
                    exit()
                elif key == ord("y"):
                    drawing_board.line = not drawing_board.line
                elif key == ord("d"):
                    color = switch_coloring_mode(color, colors.DELETE_COLOR)
                elif key == ord("a"):
                    color = switch_coloring_mode(color, colors.NOT_FIELD_COLOR)
                elif key == ord("+") or key == ord("w"):
                    drawing_board.brush_size += 2
                elif key == ord("-") or key == ord("s"):
                    if drawing_board.brush_size >= 3:
                        drawing_board.brush_size -= 2
                elif key == ord("n"):
                    cv2.imwrite(
                        os.path.join(
                            labeled_images_folder,
                            f"{file_name[:-4]}_labeled.png",
                        ),
                        overlay,
                    )
                    if top is None and bottom is not None:
                        pixels_YCrCb_bottom, pixels_BGR_bottom, y_bottom = (
                            extract_pixels(
                                overlay,
                                pixels_YCrCb_bottom,
                                pixels_BGR_bottom,
                                image_YCrCb,
                                image_BGR,
                                y_bottom,
                            )
                        )
                    elif top is not None and bottom is None:
                        pixels_YCrCb_top, pixels_BGR_top, y_top = (
                            extract_pixels(
                                overlay,
                                pixels_YCrCb_top,
                                pixels_BGR_top,
                                image_YCrCb,
                                image_BGR,
                                y_top,
                            )
                        )
                    break
                drawing_board.color = color

            cv2.destroyAllWindows()

    top_duration = 0
    bottom_duration = 0

    if len(pixels_YCrCb_top) > 0:
        start = time.time()
        model = optimize_thresholds(
            pixels_BGR_top, pixels_YCrCb_top, y_top, "top"
        )
        end = time.time()
        top_duration = end - start

    if len(pixels_YCrCb_bottom) > 0:
        start = time.time()
        model = optimize_thresholds(
            pixels_BGR_bottom, pixels_YCrCb_bottom, y_bottom, "bottom"
        )
        end = time.time()
        bottom_duration = end - start

    print("\nTraing times:")
    print(f"  - Top:    {(top_duration / 60):.2f}min")
    print(f"  - Bottom: {(bottom_duration / 60):.2f}min")
