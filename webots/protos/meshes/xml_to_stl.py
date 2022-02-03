import collections
import pathlib
import struct
import typing
import xml.etree.ElementTree

Vertex = collections.namedtuple('Vertex', ['x', 'y', 'z'])
Triangle = collections.namedtuple('Triangle', ['a', 'b', 'c'])


def extract_vertices_from_geometry(
        geometry: xml.etree.ElementTree.Element) -> typing.List[Vertex]:
    vertex_buffer = geometry.find('vertexbuffer')
    assert vertex_buffer is not None
    vertices = [
        vertex.find('position') for vertex in vertex_buffer.findall('vertex')
    ]
    assert len(vertices) == int(geometry.attrib['vertexcount'])
    assert all(vertex is not None for vertex in vertices)
    assert all(
        'x' in vertex.attrib and 'y' in vertex.attrib and 'z' in vertex.attrib
        for vertex in vertices)
    return [
        Vertex(float(vertex.attrib['x']), float(vertex.attrib['y']),
               float(vertex.attrib['z'])) for vertex in vertices
    ]


def parse_xml(
    xml_file: pathlib.Path
) -> typing.Dict[typing.Tuple[int, str], typing.List[Triangle]]:
    tree = xml.etree.ElementTree.parse(xml_file)
    root = tree.getroot()
    assert root.tag == 'mesh'
    shared_geometry = root.find('sharedgeometry')
    shared_vertices = extract_vertices_from_geometry(
        shared_geometry) if shared_geometry is not None else []
    submeshes = root.find('submeshes')
    assert submeshes is not None
    triangles: typing.Dict[typing.Tuple[int, str], typing.List[Triangle]] = {}
    for submesh_index, submesh in enumerate(submeshes.findall('submesh')):
        triangles[(submesh_index, submesh.attrib['material'])] = []
        geometry = submesh.find('geometry')
        vertices = shared_vertices
        if geometry is not None:
            vertices = extract_vertices_from_geometry(geometry)
        faces = submesh.find('faces')
        for face in faces.findall('face'):
            triangles[(submesh_index, submesh.attrib['material'])].append(
                Triangle(
                    vertices[int(face.attrib['v1'])],
                    vertices[int(face.attrib['v2'])],
                    vertices[int(face.attrib['v3'])],
                ))
    return triangles


def write_stl(stl_file: pathlib.Path, triangles: typing.List[Triangle]):
    with stl_file.open(mode='wb') as f:
        f.write(b'\x00' * 80)
        f.write(struct.pack('<I', len(triangles)))
        for triangle in triangles:
            f.write(b'\x00' * 12)
            f.write(
                struct.pack('<fff', triangle.a.x, triangle.a.y, triangle.a.z))
            f.write(
                struct.pack('<fff', triangle.b.x, triangle.b.y, triangle.b.z))
            f.write(
                struct.pack('<fff', triangle.c.x, triangle.c.y, triangle.c.z))
            f.write(b'\x00' * 2)


# read XMLs from OgreXMLExporter and convert them to STL
for xml_file in pathlib.Path('.').glob('*.xml'):
    triangles = parse_xml(xml_file)
    for (submesh_index, submesh_material), submesh_triangles in triangles.items():
        assert submesh_material.startswith('NaoMat_')
        submesh_material = submesh_material[7:]
        stl_file = xml_file.with_suffix(
            f'.{submesh_index}.{submesh_material}.stl')
        print(xml_file, '->', stl_file)
        write_stl(stl_file, submesh_triangles)
