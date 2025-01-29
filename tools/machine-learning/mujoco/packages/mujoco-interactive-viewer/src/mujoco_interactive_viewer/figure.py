from dataclasses import dataclass

import mujoco


@dataclass
class Figure:
    _figure: mujoco.MjvFigure

    def __init__(self) -> None:
        self._figure = mujoco.MjvFigure()
        mujoco.mjv_defaultFigure(self._figure)
        self._figure.flg_extend = 1

    def show_tick_labels(self, *, x: bool, y: bool) -> None:
        self._figure.flg_ticklabel[0] = x
        self._figure.flg_ticklabel[1] = y

    def line_width(self, width: float) -> None:
        self._figure.linewidth = width

    def grid_width(self, width: float) -> None:
        self._figure.gridwidth = width

    def grid_size(self, x: int, y: int) -> None:
        self._figure.gridsize[0] = x
        self._figure.gridsize[1] = y

    def grid_color(self, red: float, green: float, blue: float) -> None:
        self._figure.gridrgb[0] = red
        self._figure.gridrgb[1] = green
        self._figure.gridrgb[2] = blue

    def figure_color(
        self,
        red: float,
        green: float,
        blue: float,
        alpha: float,
    ) -> None:
        self._figure.figurergba[0] = red
        self._figure.figurergba[1] = green
        self._figure.figurergba[2] = blue
        self._figure.figurergba[3] = alpha

    def pane_color(
        self,
        red: float,
        green: float,
        blue: float,
        alpha: float,
    ) -> None:
        self._figure.panergba[0] = red
        self._figure.panergba[1] = green
        self._figure.panergba[2] = blue
        self._figure.panergba[3] = alpha

    def legend_color(
        self,
        red: float,
        green: float,
        blue: float,
        alpha: float,
    ) -> None:
        self._figure.panergba[0] = red
        self._figure.panergba[1] = green
        self._figure.panergba[2] = blue
        self._figure.panergba[3] = alpha

    def text_color(
        self,
        red: float,
        green: float,
        blue: float,
    ) -> None:
        self._figure.panergba[0] = red
        self._figure.panergba[1] = green
        self._figure.panergba[2] = blue

    def line_color(
        self,
        line_name: str,
        red: float,
        green: float,
        blue: float,
    ) -> None:
        line_id = self._line_id(line_name)
        self._figure.linergb[line_id][0] = red
        self._figure.linergb[line_id][1] = green
        self._figure.linergb[line_id][2] = blue

    def axis_range(
        self,
        x_min: float | None = None,
        x_max: float | None = None,
        y_min: float | None = None,
        y_max: float | None = None,
    ) -> None:
        if x_min is not None:
            self._figure.range[0][0] = x_min
        if x_max is not None:
            self._figure.range[0][1] = x_max
        if y_min is not None:
            self._figure.range[1][0] = y_min
        if y_max is not None:
            self._figure.range[1][1] = y_max

    def set_title(self, title: str) -> None:
        self._figure.title = title

    def set_x_label(self, label: str) -> None:
        self._figure.xlabel = label

    def add_line(self, line_name: str) -> None:
        name_bytes = line_name.encode("utf8")
        if name_bytes == b"":
            raise EmptyLineNameError()
        if name_bytes in self._figure.linename:
            raise LineNameAlreadyExistsError()

        try:
            empty_line_id = self._figure.linename.tolist().index(b"")
        except ValueError as e:
            raise NoEmptyLineSlotsError() from e

        self._figure.linename[empty_line_id] = line_name

        for i in range(mujoco.mjMAXLINEPNT):
            # line data is stored in the form [x0, y0, x1, y1, x2, y2, ...]
            self._figure.linedata[empty_line_id][2 * i] = -float(i)

    def _line_id(self, line_name: str) -> int:
        name_bytes = line_name.encode("utf8")
        try:
            line_id = self._figure.linename.tolist().index(name_bytes)
        except ValueError as e:
            raise LineNotFound() from e
        return line_id

    def push_data_to_line(
        self,
        line_name: str,
        line_data: float,
    ) -> None:
        line_id = self._line_id(line_name)

        num_points: int = self._figure.linepnt[line_id]  # type: ignore[reportAssignmentType]
        num_points = min(mujoco.mjMAXLINEPNT, num_points + 1)

        for i in range(num_points - 1, 0, -1):
            self._figure.linedata[line_id][2 * i + 1] = self._figure.linedata[
                line_id
            ][2 * i - 1]

        self._figure.linepnt[line_id] = num_points
        self._figure.linedata[line_id][1] = line_data


class EmptyLineNameError(Exception):
    def __init__(self, message: str = "line name cannot be empty") -> None:
        super().__init__(message)


class LineNameAlreadyExistsError(Exception):
    def __init__(
        self,
        message: str = "line name already exists in this figure",
    ) -> None:
        super().__init__(message)


class NoEmptyLineSlotsError(Exception):
    def __init__(
        self,
        message: str = "no empty line slots available",
    ) -> None:
        super().__init__(message)


class LineNotFound(Exception):
    def __init__(
        self,
        message: str = "line not found in this figure",
    ) -> None:
        super().__init__(message)
