from __future__ import annotations
import mujoco._enums
import mujoco._structs
import numpy
import typing
__all__ = ['MjByteVec', 'MjCharVec', 'MjOption', 'MjSpec', 'MjStatistic', 'MjStringVec', 'MjVisual', 'MjsActuator', 'MjsBody', 'MjsCamera', 'MjsCompiler', 'MjsDefault', 'MjsElement', 'MjsEquality', 'MjsExclude', 'MjsFlex', 'MjsFrame', 'MjsGeom', 'MjsHField', 'MjsJoint', 'MjsKey', 'MjsLight', 'MjsMaterial', 'MjsMesh', 'MjsNumeric', 'MjsOrientation', 'MjsPair', 'MjsPlugin', 'MjsSensor', 'MjsSite', 'MjsSkin', 'MjsTendon', 'MjsText', 'MjsTexture', 'MjsTuple', 'MjsWrap']
class MjByteVec:
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def __getitem__(self, arg0: int) -> ...:
        ...
    def __init__(self, arg0: ..., arg1: int) -> None:
        ...
    def __iter__(self) -> typing.Iterator[...]:
        ...
    def __len__(self) -> int:
        ...
    def __setitem__(self, arg0: int, arg1: ...) -> None:
        ...
class MjCharVec:
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def __getitem__(self, arg0: int) -> str:
        ...
    def __init__(self, arg0: str, arg1: int) -> None:
        ...
    def __iter__(self) -> typing.Iterator[str]:
        ...
    def __len__(self) -> int:
        ...
    def __setitem__(self, arg0: int, arg1: str) -> None:
        ...
class MjOption:
    apirate: float
    ccd_iterations: int
    ccd_tolerance: float
    cone: int
    density: float
    disableactuator: int
    disableflags: int
    enableflags: int
    impratio: float
    integrator: int
    iterations: int
    jacobian: int
    ls_iterations: int
    ls_tolerance: float
    noslip_iterations: int
    noslip_tolerance: float
    o_margin: float
    sdf_initpoints: int
    sdf_iterations: int
    solver: int
    timestep: float
    tolerance: float
    viscosity: float
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    @property
    def gravity(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @gravity.setter
    def gravity(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def magnetic(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @magnetic.setter
    def magnetic(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def o_friction(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @o_friction.setter
    def o_friction(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def o_solimp(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @o_solimp.setter
    def o_solimp(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def o_solref(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @o_solref.setter
    def o_solref(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def wind(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @wind.setter
    def wind(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
class MjSpec:
    assets: dict
    comment: str
    compiler: MjsCompiler
    hasImplicitPluginElem: int
    memory: int
    meshdir: str
    modelfiledir: str
    modelname: str
    nconmax: int
    nemax: int
    njmax: int
    nkey: int
    nstack: int
    nuser_actuator: int
    nuser_body: int
    nuser_cam: int
    nuser_geom: int
    nuser_jnt: int
    nuser_sensor: int
    nuser_site: int
    nuser_tendon: int
    nuserdata: int
    option: MjOption
    stat: MjStatistic
    strippath: int
    texturedir: str
    visual: MjVisual
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    @staticmethod
    def from_file(filename: str, assets: dict[str, bytes] | None = None) -> MjSpec:
        """
            Creates a spec from an XML file.
        
            Parameters
            ----------
            filename : str
                Path to the XML file.
            assets : dict, optional
                A dictionary of assets to be used by the spec. The keys are asset names
                and the values are asset contents.
        """
    @staticmethod
    def from_string(xml: str, assets: dict[str, bytes] | None = None) -> MjSpec:
        """
            Creates a spec from an XML string.
        
            Parameters
            ----------
            xml : str
                XML string.
            assets : dict, optional
                A dictionary of assets to be used by the spec. The keys are asset names
                and the values are asset contents.
        """
    def __init__(self) -> None:
        ...
    def add_actuator(self, default: MjsDefault = None, **kwargs) -> MjsActuator:
        ...
    def add_default(self, arg0: str, arg1: MjsDefault) -> MjsDefault:
        ...
    def add_equality(self, default: MjsDefault = None, **kwargs) -> MjsEquality:
        ...
    def add_exclude(self, **kwargs) -> MjsExclude:
        ...
    def add_flex(self, **kwargs) -> MjsFlex:
        ...
    def add_hfield(self, **kwargs) -> MjsHField:
        ...
    def add_key(self, **kwargs) -> MjsKey:
        ...
    def add_material(self, default: MjsDefault = None, **kwargs) -> MjsMaterial:
        ...
    def add_mesh(self, default: MjsDefault = None, **kwargs) -> MjsMesh:
        ...
    def add_numeric(self, **kwargs) -> MjsNumeric:
        ...
    def add_pair(self, default: MjsDefault = None, **kwargs) -> MjsPair:
        ...
    def add_plugin(self, **kwargs) -> MjsPlugin:
        ...
    def add_sensor(self, **kwargs) -> MjsSensor:
        ...
    def add_skin(self, **kwargs) -> MjsSkin:
        ...
    def add_tendon(self, default: MjsDefault = None, **kwargs) -> MjsTendon:
        ...
    def add_text(self, **kwargs) -> MjsText:
        ...
    def add_texture(self, **kwargs) -> MjsTexture:
        ...
    def add_tuple(self, **kwargs) -> MjsTuple:
        ...
    def compile(self) -> typing.Any:
        ...
    def copy(self) -> MjSpec:
        ...
    def default(self) -> MjsDefault:
        ...
    def detach_body(self, arg0: MjsBody) -> None:
        ...
    def find_actuator(self, arg0: str) -> MjsActuator:
        ...
    def find_body(self, arg0: str) -> MjsBody:
        ...
    def find_default(self, arg0: str) -> MjsDefault:
        ...
    def find_frame(self, arg0: str) -> MjsFrame:
        ...
    def find_sensor(self, arg0: str) -> MjsSensor:
        ...
    def find_site(self, arg0: str) -> MjsSite:
        ...
    def recompile(self, arg0: typing.Any, arg1: typing.Any) -> typing.Any:
        ...
    def to_xml(self) -> str:
        ...
    @property
    def actuators(self) -> list:
        ...
    @property
    def bodies(self) -> list:
        ...
    @property
    def cameras(self) -> list:
        ...
    @property
    def equalities(self) -> list:
        ...
    @property
    def excludes(self) -> list:
        ...
    @property
    def flexes(self) -> list:
        ...
    @property
    def frames(self) -> list:
        ...
    @property
    def geoms(self) -> list:
        ...
    @property
    def hfields(self) -> list:
        ...
    @property
    def joints(self) -> list:
        ...
    @property
    def keys(self) -> list:
        ...
    @property
    def lights(self) -> list:
        ...
    @property
    def materials(self) -> list:
        ...
    @property
    def meshes(self) -> list:
        ...
    @property
    def numerics(self) -> list:
        ...
    @property
    def pairs(self) -> list:
        ...
    @property
    def plugins(self) -> list:
        ...
    @property
    def sensors(self) -> list:
        ...
    @property
    def sites(self) -> list:
        ...
    @property
    def skins(self) -> list:
        ...
    @property
    def tendons(self) -> list:
        ...
    @property
    def texts(self) -> list:
        ...
    @property
    def textures(self) -> list:
        ...
    @property
    def tuples(self) -> list:
        ...
    @property
    def worldbody(self) -> MjsBody:
        ...
class MjStatistic:
    extent: float
    meaninertia: float
    meanmass: float
    meansize: float
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    @property
    def center(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @center.setter
    def center(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
class MjStringVec:
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def __getitem__(self, arg0: int) -> str:
        ...
    def __init__(self, arg0: str, arg1: int) -> None:
        ...
    def __iter__(self) -> typing.Iterator[str]:
        ...
    def __len__(self) -> int:
        ...
    def __setitem__(self, arg0: int, arg1: str) -> None:
        ...
class MjVisual:
    global: mujoco._structs.MjVisual.Global
    headlight: ...
    map: mujoco._structs.MjVisual.Map
    quality: mujoco._structs.MjVisual.Quality
    rgba: ...
    scale: mujoco._structs.MjVisual.Scale
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
class MjsActuator:
    actdim: int
    actearly: int
    actlimited: int
    biastype: mujoco._enums.mjtBias
    cranklength: float
    ctrllimited: int
    dyntype: mujoco._enums.mjtDyn
    forcelimited: int
    gaintype: mujoco._enums.mjtGain
    group: int
    info: str
    inheritrange: float
    name: str
    plugin: MjsPlugin
    refsite: str
    slidersite: str
    target: str
    trntype: mujoco._enums.mjtTrn
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    @property
    def actrange(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @actrange.setter
    def actrange(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def biasprm(self) -> numpy.ndarray[numpy.float64[10, 1], numpy.ndarray.flags.writeable]:
        ...
    @biasprm.setter
    def biasprm(self, arg1: numpy.ndarray[numpy.float64[10, 1]]) -> None:
        ...
    @property
    def ctrlrange(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @ctrlrange.setter
    def ctrlrange(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def dynprm(self) -> numpy.ndarray[numpy.float64[10, 1], numpy.ndarray.flags.writeable]:
        ...
    @dynprm.setter
    def dynprm(self, arg1: numpy.ndarray[numpy.float64[10, 1]]) -> None:
        ...
    @property
    def forcerange(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @forcerange.setter
    def forcerange(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def gainprm(self) -> numpy.ndarray[numpy.float64[10, 1], numpy.ndarray.flags.writeable]:
        ...
    @gainprm.setter
    def gainprm(self, arg1: numpy.ndarray[numpy.float64[10, 1]]) -> None:
        ...
    @property
    def gear(self) -> numpy.ndarray[numpy.float64[6, 1], numpy.ndarray.flags.writeable]:
        ...
    @gear.setter
    def gear(self, arg1: numpy.ndarray[numpy.float64[6, 1]]) -> None:
        ...
    @property
    def lengthrange(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @lengthrange.setter
    def lengthrange(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsBody:
    alt: MjsOrientation
    childclass: str
    explicitinertial: int
    gravcomp: float
    ialt: MjsOrientation
    info: str
    mass: float
    mocap: int
    name: str
    plugin: MjsPlugin
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def add_body(self, default: MjsDefault = None, **kwargs) -> MjsBody:
        ...
    def add_camera(self, default: MjsDefault = None, **kwargs) -> MjsCamera:
        ...
    def add_frame(self, default: MjsFrame = None, **kwargs) -> MjsFrame:
        ...
    def add_freejoint(self, **kwargs) -> MjsJoint:
        ...
    def add_geom(self, default: MjsDefault = None, **kwargs) -> MjsGeom:
        ...
    def add_joint(self, default: MjsDefault = None, **kwargs) -> MjsJoint:
        ...
    def add_light(self, default: MjsDefault = None, **kwargs) -> MjsLight:
        ...
    def add_site(self, default: MjsDefault = None, **kwargs) -> MjsSite:
        ...
    def attach_frame(self, frame: MjsFrame, prefix: str | None = None, suffix: str | None = None) -> MjsFrame:
        ...
    def default(self) -> MjsDefault:
        ...
    @typing.overload
    def find_all(self, arg0: mujoco._enums.mjtObj) -> list:
        ...
    @typing.overload
    def find_all(self, arg0: str) -> list:
        ...
    def find_child(self, arg0: str) -> MjsBody:
        ...
    def first_body(self) -> MjsBody:
        ...
    def first_camera(self) -> MjsCamera:
        ...
    def first_frame(self) -> MjsFrame:
        ...
    def first_geom(self) -> MjsGeom:
        ...
    def first_joint(self) -> MjsJoint:
        ...
    def first_light(self) -> MjsLight:
        ...
    def first_site(self) -> MjsSite:
        ...
    def next_body(self, arg0: MjsBody) -> MjsBody:
        ...
    def next_camera(self, arg0: MjsCamera) -> MjsCamera:
        ...
    def next_frame(self, arg0: MjsFrame) -> MjsFrame:
        ...
    def next_geom(self, arg0: MjsGeom) -> MjsGeom:
        ...
    def next_joint(self, arg0: MjsJoint) -> MjsJoint:
        ...
    def next_light(self, arg0: MjsLight) -> MjsLight:
        ...
    def next_site(self, arg0: MjsSite) -> MjsSite:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    def spec(self) -> mjSpec_:
        ...
    def to_frame(self) -> MjsFrame:
        ...
    @property
    def bodies(self) -> list:
        ...
    @property
    def cameras(self) -> list:
        ...
    @property
    def frames(self) -> list:
        ...
    @property
    def fullinertia(self) -> numpy.ndarray[numpy.float64[6, 1], numpy.ndarray.flags.writeable]:
        ...
    @fullinertia.setter
    def fullinertia(self, arg1: numpy.ndarray[numpy.float64[6, 1]]) -> None:
        ...
    @property
    def geoms(self) -> list:
        ...
    @property
    def inertia(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @inertia.setter
    def inertia(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def ipos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @ipos.setter
    def ipos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def iquat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @iquat.setter
    def iquat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def joints(self) -> list:
        ...
    @property
    def lights(self) -> list:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def quat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @quat.setter
    def quat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def sites(self) -> list:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsCamera:
    alt: MjsOrientation
    fovy: float
    info: str
    ipd: float
    mode: mujoco._enums.mjtCamLight
    name: str
    orthographic: int
    targetbody: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def focal_length(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @focal_length.setter
    def focal_length(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def focal_pixel(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @focal_pixel.setter
    def focal_pixel(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def intrinsic(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @intrinsic.setter
    def intrinsic(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def principal_length(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @principal_length.setter
    def principal_length(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def principal_pixel(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @principal_pixel.setter
    def principal_pixel(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def quat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @quat.setter
    def quat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def resolution(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @resolution.setter
    def resolution(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def sensor_size(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @sensor_size.setter
    def sensor_size(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsCompiler:
    LRopt: mujoco._structs.MjLROpt
    alignfree: int
    autolimits: int
    balanceinertia: int
    boundinertia: float
    boundmass: float
    degree: int
    discardvisual: int
    fitaabb: int
    fusestatic: int
    inertiafromgeom: int
    settotalmass: float
    usethread: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    @property
    def eulerseq(self) -> MjCharVec:
        ...
    @eulerseq.setter
    def eulerseq(self, arg1: typing.Any) -> None:
        ...
    @property
    def inertiagrouprange(self) -> numpy.ndarray[numpy.int32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @inertiagrouprange.setter
    def inertiagrouprange(self, arg1: numpy.ndarray[numpy.int32[2, 1]]) -> None:
        ...
class MjsDefault:
    actuator: MjsActuator
    camera: MjsCamera
    equality: MjsEquality
    flex: MjsFlex
    geom: MjsGeom
    joint: MjsJoint
    light: MjsLight
    material: MjsMaterial
    mesh: MjsMesh
    name: str
    pair: MjsPair
    site: MjsSite
    tendon: MjsTendon
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
class MjsElement:
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
class MjsEquality:
    active: int
    info: str
    name: str
    name1: str
    name2: str
    objtype: mujoco._enums.mjtObj
    type: mujoco._enums.mjtEq
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    @property
    def data(self) -> numpy.ndarray[numpy.float64[11, 1], numpy.ndarray.flags.writeable]:
        ...
    @data.setter
    def data(self, arg1: numpy.ndarray[numpy.float64[11, 1]]) -> None:
        ...
    @property
    def solimp(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp.setter
    def solimp(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref.setter
    def solref(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
class MjsExclude:
    bodyname1: str
    bodyname2: str
    info: str
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
class MjsFlex:
    activelayers: int
    conaffinity: int
    condim: int
    contype: int
    damping: float
    dim: int
    edgedamping: float
    edgestiffness: float
    flatskin: int
    gap: float
    group: int
    info: str
    internal: int
    margin: float
    material: str
    name: str
    poisson: float
    priority: int
    radius: float
    selfcollide: int
    solmix: float
    thickness: float
    young: float
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def elem(self) -> numpy.ndarray[numpy.int32]:
        ...
    @elem.setter
    def elem(self, arg1: typing.Any) -> None:
        ...
    @property
    def friction(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @friction.setter
    def friction(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def solimp(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp.setter
    def solimp(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref.setter
    def solref(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def texcoord(self) -> numpy.ndarray[numpy.float32]:
        ...
    @texcoord.setter
    def texcoord(self, arg1: typing.Any) -> None:
        ...
    @property
    def vert(self) -> numpy.ndarray[numpy.float64]:
        ...
    @vert.setter
    def vert(self, arg1: typing.Any) -> None:
        ...
    @property
    def vertbody(self) -> MjStringVec:
        ...
    @vertbody.setter
    def vertbody(self, arg1: typing.Any) -> None:
        ...
class MjsFrame:
    alt: MjsOrientation
    childclass: str
    info: str
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def attach(self, spec: MjSpec, prefix: str | None = None, suffix: str | None = None) -> MjsFrame:
        ...
    def attach_body(self, body: MjsBody, prefix: str | None = None, suffix: str | None = None) -> MjsBody:
        ...
    def delete(self) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def quat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @quat.setter
    def quat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
class MjsGeom:
    alt: MjsOrientation
    conaffinity: int
    condim: int
    contype: int
    density: float
    fitscale: float
    fluid_ellipsoid: float
    gap: float
    group: int
    hfieldname: str
    info: str
    margin: float
    mass: float
    material: str
    meshname: str
    name: str
    plugin: MjsPlugin
    priority: int
    solmix: float
    type: mujoco._enums.mjtGeom
    typeinertia: mujoco._enums.mjtGeomInertia
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def fluid_coefs(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @fluid_coefs.setter
    def fluid_coefs(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def friction(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @friction.setter
    def friction(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def fromto(self) -> numpy.ndarray[numpy.float64[6, 1], numpy.ndarray.flags.writeable]:
        ...
    @fromto.setter
    def fromto(self, arg1: numpy.ndarray[numpy.float64[6, 1]]) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def quat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @quat.setter
    def quat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def size(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @size.setter
    def size(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def solimp(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp.setter
    def solimp(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref.setter
    def solref(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsHField:
    content_type: str
    file: str
    info: str
    name: str
    ncol: int
    nrow: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def size(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @size.setter
    def size(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float32]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsJoint:
    actfrclimited: int
    actgravcomp: int
    align: int
    armature: float
    damping: float
    frictionloss: float
    group: int
    info: str
    limited: int
    margin: float
    name: str
    ref: float
    springref: float
    stiffness: float
    type: mujoco._enums.mjtJoint
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def actfrcrange(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @actfrcrange.setter
    def actfrcrange(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def axis(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @axis.setter
    def axis(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def range(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @range.setter
    def range(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def solimp_friction(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp_friction.setter
    def solimp_friction(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solimp_limit(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp_limit.setter
    def solimp_limit(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref_friction(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref_friction.setter
    def solref_friction(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def solref_limit(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref_limit.setter
    def solref_limit(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def springdamper(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @springdamper.setter
    def springdamper(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsKey:
    info: str
    name: str
    time: float
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def act(self) -> numpy.ndarray[numpy.float64]:
        ...
    @act.setter
    def act(self, arg1: typing.Any) -> None:
        ...
    @property
    def ctrl(self) -> numpy.ndarray[numpy.float64]:
        ...
    @ctrl.setter
    def ctrl(self, arg1: typing.Any) -> None:
        ...
    @property
    def mpos(self) -> numpy.ndarray[numpy.float64]:
        ...
    @mpos.setter
    def mpos(self, arg1: typing.Any) -> None:
        ...
    @property
    def mquat(self) -> numpy.ndarray[numpy.float64]:
        ...
    @mquat.setter
    def mquat(self, arg1: typing.Any) -> None:
        ...
    @property
    def qpos(self) -> numpy.ndarray[numpy.float64]:
        ...
    @qpos.setter
    def qpos(self, arg1: typing.Any) -> None:
        ...
    @property
    def qvel(self) -> numpy.ndarray[numpy.float64]:
        ...
    @qvel.setter
    def qvel(self, arg1: typing.Any) -> None:
        ...
class MjsLight:
    active: int
    bulbradius: float
    castshadow: int
    cutoff: float
    directional: int
    exponent: float
    info: str
    mode: mujoco._enums.mjtCamLight
    name: str
    targetbody: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def ambient(self) -> numpy.ndarray[numpy.float32[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @ambient.setter
    def ambient(self, arg1: numpy.ndarray[numpy.float32[3, 1]]) -> None:
        ...
    @property
    def attenuation(self) -> numpy.ndarray[numpy.float32[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @attenuation.setter
    def attenuation(self, arg1: numpy.ndarray[numpy.float32[3, 1]]) -> None:
        ...
    @property
    def diffuse(self) -> numpy.ndarray[numpy.float32[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @diffuse.setter
    def diffuse(self, arg1: numpy.ndarray[numpy.float32[3, 1]]) -> None:
        ...
    @property
    def dir(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @dir.setter
    def dir(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def specular(self) -> numpy.ndarray[numpy.float32[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @specular.setter
    def specular(self, arg1: numpy.ndarray[numpy.float32[3, 1]]) -> None:
        ...
class MjsMaterial:
    emission: float
    info: str
    metallic: float
    name: str
    reflectance: float
    roughness: float
    shininess: float
    specular: float
    texuniform: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def texrepeat(self) -> numpy.ndarray[numpy.float32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @texrepeat.setter
    def texrepeat(self, arg1: numpy.ndarray[numpy.float32[2, 1]]) -> None:
        ...
    @property
    def textures(self) -> MjStringVec:
        ...
    @textures.setter
    def textures(self, arg1: typing.Any) -> None:
        ...
class MjsMesh:
    content_type: str
    file: str
    inertia: mujoco._enums.mjtMeshInertia
    info: str
    maxhullvert: int
    name: str
    plugin: MjsPlugin
    smoothnormal: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    @property
    def refpos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @refpos.setter
    def refpos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def refquat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @refquat.setter
    def refquat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def scale(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @scale.setter
    def scale(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def userface(self) -> numpy.ndarray[numpy.int32]:
        ...
    @userface.setter
    def userface(self, arg1: typing.Any) -> None:
        ...
    @property
    def userfacetexcoord(self) -> numpy.ndarray[numpy.int32]:
        ...
    @userfacetexcoord.setter
    def userfacetexcoord(self, arg1: typing.Any) -> None:
        ...
    @property
    def usernormal(self) -> numpy.ndarray[numpy.float32]:
        ...
    @usernormal.setter
    def usernormal(self, arg1: typing.Any) -> None:
        ...
    @property
    def usertexcoord(self) -> numpy.ndarray[numpy.float32]:
        ...
    @usertexcoord.setter
    def usertexcoord(self, arg1: typing.Any) -> None:
        ...
    @property
    def uservert(self) -> numpy.ndarray[numpy.float32]:
        ...
    @uservert.setter
    def uservert(self, arg1: typing.Any) -> None:
        ...
class MjsNumeric:
    info: str
    name: str
    size: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def data(self) -> numpy.ndarray[numpy.float64]:
        ...
    @data.setter
    def data(self, arg1: typing.Any) -> None:
        ...
class MjsOrientation:
    type: mujoco._enums.mjtOrientation
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    @property
    def axisangle(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @axisangle.setter
    def axisangle(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def euler(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @euler.setter
    def euler(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def xyaxes(self) -> numpy.ndarray[numpy.float64[6, 1], numpy.ndarray.flags.writeable]:
        ...
    @xyaxes.setter
    def xyaxes(self, arg1: numpy.ndarray[numpy.float64[6, 1]]) -> None:
        ...
    @property
    def zaxis(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @zaxis.setter
    def zaxis(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
class MjsPair:
    condim: int
    gap: float
    geomname1: str
    geomname2: str
    info: str
    margin: float
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    @property
    def friction(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @friction.setter
    def friction(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solimp(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp.setter
    def solimp(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref.setter
    def solref(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def solreffriction(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solreffriction.setter
    def solreffriction(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
class MjsPlugin:
    active: int
    info: str
    name: str
    plugin_name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def id(self) -> int:
        ...
    @id.setter
    def id(self, arg1: MjsPlugin) -> None:
        ...
class MjsSensor:
    cutoff: float
    datatype: mujoco._enums.mjtDataType
    dim: int
    info: str
    name: str
    needstage: mujoco._enums.mjtStage
    noise: float
    objname: str
    objtype: mujoco._enums.mjtObj
    plugin: MjsPlugin
    refname: str
    reftype: mujoco._enums.mjtObj
    type: mujoco._enums.mjtSensor
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsSite:
    alt: MjsOrientation
    group: int
    info: str
    material: str
    name: str
    type: mujoco._enums.mjtGeom
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def attach(self, body: MjSpec, prefix: str | None = None, suffix: str | None = None) -> MjsFrame:
        ...
    def attach_body(self, body: MjsBody, prefix: str | None = None, suffix: str | None = None) -> MjsBody:
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def set_frame(self, arg0: MjsFrame) -> None:
        ...
    @property
    def fromto(self) -> numpy.ndarray[numpy.float64[6, 1], numpy.ndarray.flags.writeable]:
        ...
    @fromto.setter
    def fromto(self, arg1: numpy.ndarray[numpy.float64[6, 1]]) -> None:
        ...
    @property
    def pos(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @pos.setter
    def pos(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def quat(self) -> numpy.ndarray[numpy.float64[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @quat.setter
    def quat(self, arg1: numpy.ndarray[numpy.float64[4, 1]]) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def size(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @size.setter
    def size(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsSkin:
    file: str
    group: int
    inflate: float
    info: str
    material: str
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def bindpos(self) -> numpy.ndarray[numpy.float32]:
        ...
    @bindpos.setter
    def bindpos(self, arg1: typing.Any) -> None:
        ...
    @property
    def bindquat(self) -> numpy.ndarray[numpy.float32]:
        ...
    @bindquat.setter
    def bindquat(self, arg1: typing.Any) -> None:
        ...
    @property
    def bodyname(self) -> MjStringVec:
        ...
    @bodyname.setter
    def bodyname(self, arg1: typing.Any) -> None:
        ...
    @property
    def face(self) -> numpy.ndarray[numpy.int32]:
        ...
    @face.setter
    def face(self, arg1: typing.Any) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def texcoord(self) -> numpy.ndarray[numpy.float32]:
        ...
    @texcoord.setter
    def texcoord(self, arg1: typing.Any) -> None:
        ...
    @property
    def vert(self) -> numpy.ndarray[numpy.float32]:
        ...
    @vert.setter
    def vert(self, arg1: typing.Any) -> None:
        ...
    @property
    def vertid(self) -> list:
        ...
    @vertid.setter
    def vertid(self, arg1: typing.Any) -> None:
        ...
    @property
    def vertweight(self) -> list:
        ...
    @vertweight.setter
    def vertweight(self, arg1: typing.Any) -> None:
        ...
class MjsTendon:
    damping: float
    frictionloss: float
    group: int
    info: str
    limited: int
    margin: float
    material: str
    name: str
    stiffness: float
    width: float
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def default(self) -> MjsDefault:
        ...
    def delete(self) -> None:
        ...
    def set_default(self, arg0: MjsDefault) -> None:
        ...
    def wrap_geom(self, arg0: str, arg1: str) -> MjsWrap:
        ...
    def wrap_joint(self, arg0: str, arg1: float) -> MjsWrap:
        ...
    def wrap_pulley(self, arg0: float) -> MjsWrap:
        ...
    def wrap_site(self, arg0: str) -> MjsWrap:
        ...
    @property
    def range(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @range.setter
    def range(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def rgba(self) -> numpy.ndarray[numpy.float32[4, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgba.setter
    def rgba(self, arg1: numpy.ndarray[numpy.float32[4, 1]]) -> None:
        ...
    @property
    def solimp_friction(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp_friction.setter
    def solimp_friction(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solimp_limit(self) -> numpy.ndarray[numpy.float64[5, 1], numpy.ndarray.flags.writeable]:
        ...
    @solimp_limit.setter
    def solimp_limit(self, arg1: numpy.ndarray[numpy.float64[5, 1]]) -> None:
        ...
    @property
    def solref_friction(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref_friction.setter
    def solref_friction(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def solref_limit(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @solref_limit.setter
    def solref_limit(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def springlength(self) -> numpy.ndarray[numpy.float64[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @springlength.setter
    def springlength(self, arg1: numpy.ndarray[numpy.float64[2, 1]]) -> None:
        ...
    @property
    def userdata(self) -> numpy.ndarray[numpy.float64]:
        ...
    @userdata.setter
    def userdata(self, arg1: typing.Any) -> None:
        ...
class MjsText:
    data: str
    info: str
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
class MjsTexture:
    builtin: int
    content_type: str
    file: str
    height: int
    hflip: int
    info: str
    mark: int
    name: str
    nchannel: int
    random: float
    type: mujoco._enums.mjtTexture
    vflip: int
    width: int
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def cubefiles(self) -> MjStringVec:
        ...
    @cubefiles.setter
    def cubefiles(self, arg1: typing.Any) -> None:
        ...
    @property
    def data(self) -> MjByteVec:
        ...
    @data.setter
    def data(self, arg1: bytes) -> None:
        ...
    @property
    def gridlayout(self) -> MjCharVec:
        ...
    @gridlayout.setter
    def gridlayout(self, arg1: typing.Any) -> None:
        ...
    @property
    def gridsize(self) -> numpy.ndarray[numpy.int32[2, 1], numpy.ndarray.flags.writeable]:
        ...
    @gridsize.setter
    def gridsize(self, arg1: numpy.ndarray[numpy.int32[2, 1]]) -> None:
        ...
    @property
    def markrgb(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @markrgb.setter
    def markrgb(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def rgb1(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgb1.setter
    def rgb1(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
    @property
    def rgb2(self) -> numpy.ndarray[numpy.float64[3, 1], numpy.ndarray.flags.writeable]:
        ...
    @rgb2.setter
    def rgb2(self, arg1: numpy.ndarray[numpy.float64[3, 1]]) -> None:
        ...
class MjsTuple:
    info: str
    name: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
    def delete(self) -> None:
        ...
    @property
    def objname(self) -> MjStringVec:
        ...
    @objname.setter
    def objname(self, arg1: typing.Any) -> None:
        ...
    @property
    def objprm(self) -> numpy.ndarray[numpy.float64]:
        ...
    @objprm.setter
    def objprm(self, arg1: typing.Any) -> None:
        ...
    @property
    def objtype(self) -> numpy.ndarray[numpy.int32]:
        ...
    @objtype.setter
    def objtype(self, arg1: typing.Any) -> None:
        ...
class MjsWrap:
    info: str
    @staticmethod
    def _pybind11_conduit_v1_(*args, **kwargs):
        ...
