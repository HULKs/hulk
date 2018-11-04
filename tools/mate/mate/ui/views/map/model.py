from enum import Enum
from mate.ui.views.map.layer.field import Field
from mate.ui.views.map.layer.field_config import FieldConfig
from mate.ui.views.map.layer.coordinateSystem import CoordinateSystem
from mate.ui.views.map.layer.coordinateSystem_config import CoordinateSystemConfig
from mate.ui.views.map.layer.pose import Pose
from mate.ui.views.map.layer.pose_config import PoseConfig
from mate.ui.views.map.layer.ballPosition import BallPosition
from mate.ui.views.map.layer.ballPosition_config import BallPositionConfig
from mate.ui.views.map.layer.ukf import UKF
from mate.ui.views.map.layer.ukf_config import UKFConfig
from mate.ui.views.map.layer.teamPlayers import TeamPlayers
from mate.ui.views.map.layer.teamPlayers_config import TeamPlayersConfig
from mate.ui.views.map.layer.lineData import LineData
from mate.ui.views.map.layer.lineData_config import LineDataConfig
from mate.ui.views.map.layer.sonarSensors import SonarSensors
from mate.ui.views.map.layer.sonarSensors_config import SonarSensorsConfig
from mate.ui.views.map.layer.motionPlanner import MotionPlanner
from mate.ui.views.map.layer.motionPlanner_config import MotionPlannerConfig
from mate.ui.views.map.layer.selfPlayer import SelfPlayer
from mate.ui.views.map.layer.selfPlayer_config import SelfPlayerConfig
from mate.ui.views.map.layer.obstacleData import ObstacleData
from mate.ui.views.map.layer.obstacleData_config import ObstacleDataConfig
from mate.ui.views.map.layer.ballSearch import BallSearch
from mate.ui.views.map.layer.ballSearch_config import BallSearchConfig
from mate.ui.views.map.layer.striker import Striker
from mate.ui.views.map.layer.striker_config import StrikerConfig


class TabType(Enum):
    map = 0
    layer = 1
    config = 2


# LayerType dict:
# "layer_name": (lambda: createLayerView, lambda CreateLayerPainter)
LayerType = {
        "field": (
            lambda map_model, parent, update_callback, nao:
                FieldConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                Field(layer, nao)),
        "coordinateSystem": (
            lambda map_model, parent, update_callback, nao:
                CoordinateSystemConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                CoordinateSystem(layer, nao)),
        "pose": (
            lambda map_model, parent, update_callback, nao:
                PoseConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                Pose(layer, nao)),
        "ballPosition": (
            lambda map_model, parent, update_callback, nao:
                BallPositionConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                BallPosition(layer, nao)),
        "ukf": (
            lambda map_model, parent, update_callback, nao:
                UKFConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                UKF(layer, nao)),
        "teamPlayers": (
            lambda map_model, parent, update_callback, nao:
                TeamPlayersConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                TeamPlayers(layer, nao)),
        "lineData": (
            lambda map_model, parent, update_callback, nao:
                LineDataConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                LineData(layer, nao)),
        "sonarSensors": (
            lambda map_model, parent, update_callback, nao:
                SonarSensorsConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                SonarSensors(layer, nao)),
        "motionPlanner": (
            lambda map_model, parent, update_callback, nao:
                MotionPlannerConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                MotionPlanner(layer, nao)),
        "selfPlayer": (
            lambda map_model, parent, update_callback, nao:
                SelfPlayerConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                SelfPlayer(layer, nao)),
        "obstacleData": (
            lambda map_model, parent, update_callback, nao:
                ObstacleDataConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                ObstacleData(layer, nao)),
        "ballSearch": (
            lambda map_model, parent, update_callback, nao:
                BallSearchConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                BallSearch(layer, nao)),
        "striker": (
            lambda map_model, parent, update_callback, nao:
                StrikerConfig(map_model, parent, update_callback, nao),
            lambda layer, nao:
                Striker(layer, nao)
            )
        }


class MapModel:
    def __init__(self):
        self.layer = []
        self.selected_tab = TabType.config
        self.selected_index = None
        self.viewport = [10.4, 7.4]
        self.fps = 30

    def add_layer(self, layer_type: str):
        self.layer.append({
            "type": layer_type,
            "settings": None,
            "name": layer_type,
            "enabled": True
        })

    def select_layer(self, index: int):
        self.selected_index = index

    def select_tab(self, index: int):
        self.selected_tab = TabType(index)

    def get_selected_layer(self):
        if self.selected_index < len(self.layer):
            return self.layer[self.selected_index]
        else:
            return None

    def swap_layer(self, index_a: int, index_b: int):
        self.layer[index_a], self.layer[index_b] = self.layer[index_b], \
                                                   self.layer[index_a]
