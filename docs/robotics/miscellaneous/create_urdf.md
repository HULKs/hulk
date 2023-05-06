# Create URDF and PROTO for NAOv6

For Webots we need a PROTO file which should contain 3D models and the scene graph of the NAOv6. The only source is a URDF from http://doc.aldebaran.com/2-8/family/nao_technical/kinematics_naov6.html#naov6-urdf-files. Meshes in OGRE mesh format can be found in e.g. the "C++ SDK" from https://developer.softbankrobotics.com/nao6/downloads/nao6-downloads-linux (in `share/alrobotmodel/meshes`). The URDF and meshes need to be converted to Webots PROTO.

Download and build target `OgreXMLConverter` in https://github.com/OGRECave/ogre (e.g. `cmake --build build --target OgreXMLConverter`). `OgreXMLConverter` is able to convert OGRE mesh files into XML files containing the raw vertices and face vector indices. The resulting XML files can be converted into binary STL files with the script [`xml_to_stl.py`](https://github.com/HULKs/hulk/blob/main/webots/protos/meshes/xml_to_stl.py).

The script above generates multiple STL files for each submesh contained in the XML file. This allows to set different materials in the URDF. The material's name is included in the STL filename. Since the URDF only references the old mesh files it needs to be adapted to contain multiple `<visual>` sections with same translation and rotation but with different STL mesh files and materials (`package://` prefixes can be dropped).

Cameras can be added with e.g.:

```xml
<gazebo reference="CameraTop">
  <sensor type="camera" name="CameraTop">
    <camera name="CameraTop">
      <horizontal_fov>0.982122222</horizontal_fov>
      <image>
        <width>640</width>
        <height>480</height>
        <format>R8G8B8A8</format>
      </image>
    </camera>
  </sensor>
</gazebo>
```

And an IMU with e.g.:

```xml
<gazebo reference="Accelerometer">
  <plugin filename="libgazebo_ros_imu.so">
    <topicName>IMU</topicName>
  </plugin>
</gazebo>
```

The URDF is as complete as it can get. The URDF with STLs can be converted with https://github.com/cyberbotics/urdf2webots to a PROTO file. `urdf2webots` only converts a subset of sensors (https://github.com/cyberbotics/urdf2webots/blob/6630d9778af064983f97ef1b2ea87f91c1efb48b/urdf2webots/parserURDF.py#L986-L1080) and has only limited conversion capabilities for meshes etc. At this point the PROTO file needs to be finalized manually.

Cameras are wrongly oriented and need to be rotated: Both camera's rotation needs to be `rotation 0.577350 -0.577350 -0.577350 2.093333333`.

If not setting all initial joint angles to zero, the robot seems to have random initial angles in Webots. Therefore specify `--init-pos="[0.0, 0.0, 0.0, ..., 0.0, 0.0]"` to `urdf2webots` (the amount of `0.0` corresponds to the number of joints, e.g. 130).
