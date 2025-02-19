from pathlib import Path
from xml.etree import ElementTree
from dataclasses import dataclass


@dataclass
class Vec3:
    x: float
    y: float
    z: float


@dataclass
class Vertex:
    position: Vec3 | None
    normal: Vec3 | None


@dataclass
class Face:
    a: int
    b: int
    c: int


@dataclass
class Submesh:
    material_name: str
    vertices: list[Vertex]
    faces: list[Face]


# @dataclass
# class SubmeshName:
#     name: str | None
#     index: int


def vec3_from_element(element: ElementTree.Element) -> Vec3:
    assert "x" in element.attrib and "y" in element.attrib and "z" in element.attrib
    return Vec3(
        x=float(element.attrib["x"]),
        y=float(element.attrib["y"]),
        z=float(element.attrib["z"]),
    )


def vertex_from_element(element: ElementTree.Element) -> Vertex:
    position = element.find("position")
    normal = element.find("normal")
    return Vertex(
        position=vec3_from_element(position) if position is not None else None,
        normal=vec3_from_element(normal) if normal is not None else None,
    )


def vertices_from_geometry(
    geometry: ElementTree.Element,
) -> list[Vertex]:
    vertex_buffer = geometry.find("vertexbuffer")
    assert vertex_buffer is not None
    vertices = [
        vertex_from_element(vertex) for vertex in vertex_buffer.findall("vertex")
    ]
    assert len(vertices) == int(geometry.attrib["vertexcount"])
    return vertices


def faces_from_element(element: ElementTree.Element) -> list[Face]:
    faces = element.find("faces")
    assert faces is not None

    return [
        Face(
            a=int(face.attrib["v1"]),
            b=int(face.attrib["v2"]),
            c=int(face.attrib["v3"]),
        )
        for face in faces
    ]


def submesh_from_element(
    element: ElementTree.Element,
    shared_vertices: list[Vertex],
) -> Submesh:
    geometry = element.find("geometry")
    if geometry is not None:
        vertices = vertices_from_geometry(geometry)
    else:
        vertices = shared_vertices
    material_name = element.attrib["material"]
    return Submesh(
        material_name=material_name,
        vertices=vertices,
        faces=faces_from_element(element),
    )


# def submesh_name_from_element(element: ElementTree.Element) -> SubmeshName:
#     assert "name" in element.attrib and "index" in element.attrib
#     return SubmeshName(
#         name=element.attrib["name"],
#         index=int(element.attrib["index"]),
#     )


def parse_ogre_xml(xml_file: Path) -> list[Submesh]:
    tree = ElementTree.parse(xml_file)
    root = tree.getroot()
    assert root.tag == "mesh"

    shared_geometry = root.find("sharedgeometry")
    if shared_geometry is not None:
        shared_vertices = vertices_from_geometry(shared_geometry)
    else:
        shared_vertices = []

    # XMLs are broken: not everything has a subname
    # submesh_names = root.find("submeshnames")
    # if submesh_names is not None:
    #     names = [
    #         submesh_name_from_element(submesh_name) for submesh_name in submesh_names
    #     ]
    # else:
    #     names = [SubmeshName(name=None, index=0)]

    submeshes = root.find("submeshes")
    assert submeshes is not None
    return [submesh_from_element(submesh, shared_vertices) for submesh in submeshes]


# def write_stl(stl_file: Path, triangles: list[Triangle]):
#     with stl_file.open(mode="wb") as f:
#         f.write(b"\x00" * 80)
#         f.write(struct.pack("<I", len(triangles)))
#         for triangle in triangles:
#             f.write(b"\x00" * 12)
#             f.write(struct.pack("<fff", triangle.a.x, triangle.a.y, triangle.a.z))
#             f.write(struct.pack("<fff", triangle.b.x, triangle.b.y, triangle.b.z))
#             f.write(struct.pack("<fff", triangle.c.x, triangle.c.y, triangle.c.z))
#             f.write(b"\x00" * 2)


def ogre_to_obj():
    for xml_file in Path(".").glob("*.xml"):
        print(f"\nConverting {xml_file}")
        meshes = parse_ogre_xml(xml_file)
        for index, mesh in enumerate(meshes):
            file_name = f"{xml_file.stem}.{index}.{mesh.material_name}.obj"
            print(f"Writing {file_name}")
            obj_path = xml_file.parent / file_name
            with obj_path.open(mode="w") as f:
                f.write(f"o {file_name}\n")
                f.write("\n\n# Vertices\n")
                for vertex in mesh.vertices:
                    if vertex.position is not None:
                        f.write(
                            f"v {vertex.position.x} {vertex.position.y} {vertex.position.z}\n"
                        )
                    if vertex.normal is not None:
                        f.write(
                            f"vn {vertex.normal.x} {vertex.normal.y} {vertex.normal.z}\n"
                        )
                f.write("\n\n# Faces\n")
                for face in mesh.faces:
                    f.write(
                        f"f {face.a + 1}//{face.a + 1} {face.b + 1}//{face.b + 1} {face.c + 1}//{face.c + 1}\n"
                    )


if __name__ == "__main__":
    ogre_to_obj()
