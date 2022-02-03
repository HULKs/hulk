#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <cmath>
#include <optional>

class CameraMatrix : public DataType<CameraMatrix>
{
public:
  /// the name of this DataType
  DataTypeName name__{"CameraMatrix"};
  /// a transformation matrix that describes the camera to the robot coordinate system - updated
  /// every cycle
  KinematicMatrix camera2ground;
  /// the inverse camera2ground for performance reasons - updated every cycle
  KinematicMatrix camera2groundInv;
  /// fixed cam2ground for stand pose
  KinematicMatrix cam2groundStand;
  /// a transformation matrix that describes the camera to the robot torso coordinate system -
  /// updated every cycle
  KinematicMatrix camera2torso;
  /// the inverse camera2torso for performance reasons - updated every cycle
  KinematicMatrix camera2torsoInv;
  /// the focal length with compensation for pixel size
  Vector2f fc = Vector2f::Zero();
  /// the optical center in pixel coordinates
  Vector2f cc = Vector2f::Zero();
  /// coefficients for a linear equation through the horizon: y(x) = a*x + b
  float horizonA, horizonB;
  /// whether the camera matrix is valid
  bool valid = false;
  /// the field of view of the nao
  Vector2f fov = Vector2f::Zero();

  /**
   * @brief pixelToCamera transforms pixel coordinates to camera coordinates using a pinhole camera
   * model
   * @param pixel_coordinates the pixel coordinates
   * @param cameraCoordinates the result is stored here
   */
  Vector3f pixelToCamera(const Vector2i& pixelCoordinates) const
  {
    // Note that the x axis of the camera is the the z axis of the pixel coordinate system.
    // inverse pinhole projection
    return Vector3f{1.f, (cc.x() - static_cast<float>(pixelCoordinates.x())) / fc.x(),
                    (cc.y() - static_cast<float>(pixelCoordinates.y())) / fc.y()};
  }
  /**
   * @brief cameraToPixel transforms camera coordinates to pixel coordinates
   * @param cameraCoordinates the camera coordinates
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2i> cameraToPixel(const Vector3f& cameraCoordinates) const
  {
    // A position behind the camera cannot be transformed to pixel coordinates as it does not
    // intersect the image plane.
    if (cameraCoordinates.x() <= 0.f)
    {
      return {};
    }
    // pinhole projection
    // add 0.5 for mathematical rounding
    return Vector2i{cc.x() - fc.x() * cameraCoordinates.y() / cameraCoordinates.x() + 0.5,
                    cc.y() - fc.y() * cameraCoordinates.z() / cameraCoordinates.x() + 0.5};
  }
  /**
   * @brief pixelToRobot calculates the coordinates (on ground) in the robot coordinate system of a
   * given pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param robotCoordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2f> pixelToRobot(const Vector2i& pixelCoordinates) const
  {
    return pixelToRobot(pixelCoordinates, camera2ground);
  }

  std::optional<Vector2f> pixelToRobot(const Vector2i& pixelCoordinates,
                                       const KinematicMatrix& cam2ground) const
  {
    // apply inverse projection - This results in a ray of potential points in homogeneous
    // coordinates.
    // rotate this ray to the robot coordinate system
    const Vector3f cameraCoordinates = cam2ground.rotM * pixelToCamera(pixelCoordinates);
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (cameraCoordinates.z() == 0.f || std::isnan(cameraCoordinates.x()) ||
        std::isnan(cameraCoordinates.y()) || std::isnan(cameraCoordinates.z()))
    {
      return {};
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    return Vector2f{
        cam2ground.posV.x() - cam2ground.posV.z() * cameraCoordinates.x() / cameraCoordinates.z(),
        cam2ground.posV.y() - cam2ground.posV.z() * cameraCoordinates.y() / cameraCoordinates.z()};
  }
  /**
   * @brief pixelToRobotWithZ calculates the coordinates in the robot coordinate system of a given
   * pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param z a predetermined z coordinate of the plane in which to project the pixel coordinates
   * @param robotCoordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2f> pixelToRobotWithZ(const Vector2i& pixelCoordinates, float z) const
  {
    // apply inverse projection - This results in a ray of potential points in homogeneous
    // coordinates.
    // rotate this ray to the robot coordinate system
    const Vector3f cameraCoordinates = camera2ground.rotM * pixelToCamera(pixelCoordinates);
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (cameraCoordinates.z() == 0.f)
    {
      return {};
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    return Vector2f{camera2ground.posV.x() - (camera2ground.posV.z() - z) * cameraCoordinates.x() /
                                                 cameraCoordinates.z(),
                    camera2ground.posV.y() - (camera2ground.posV.z() - z) * cameraCoordinates.y() /
                                                 cameraCoordinates.z()};
  }
  /**
   * @brief robotToPixel calculates the pixel coordinates of a given point (on ground) in robot
   * coordinates
   * @param robotCoordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2i> robotToPixel(const Vector2f& robotCoordinates) const
  {
    return robotToPixel(robotCoordinates, camera2groundInv);
  }
  std::optional<Vector2i> robotToPixel(const Vector2f& robotCoordinates,
                                       const KinematicMatrix& cam2groundInv) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f cameraCoordinates(cam2groundInv *
                               Vector3f(robotCoordinates.x(), robotCoordinates.y(), 0));
    // do pinhole projection
    return cameraToPixel(cameraCoordinates);
  }

  /**
   * @brief robotWithZToPixel calculates the pixel coordinates of a given point in robot coordinates
   * @param robotCoordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2i> robotWithZToPixel(const Vector3f& robotCoordinates) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f cameraCoordinates(camera2groundInv * robotCoordinates);
    // do pinhole projection
    return cameraToPixel(cameraCoordinates);
  }
  /**
   * @brief robotToPixel calculates the pixel coordinates of a given point in robot coordinates
   * @param torso_coordinates world coordinates
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  std::optional<Vector2i> torsoToPixel(const Vector3f& torsoCoordinates) const
  {
    // calculate camera coordinates from robot coordinates
    const Vector3f cameraCoordinates = camera2torsoInv * torsoCoordinates;
    // do pinhole projection
    return cameraToPixel(cameraCoordinates);
  }
  /**
   * @brief getPixelRadius finds out the radius in pixel coordinates that a circle at some point in
   * pixel coordinates would have
   * @param resolution the current camera resolution in px
   * @param pixel_coordinates a point in pixel coordinates (in 422 if 422 resolution) where the size
   * of an object should be calculated
   * @param robot_radius the known radius in robot coordinates
   * @param pixel_radius the estimates radius in pixel coordinates
   * @return true iff the projection was actually possible
   */
  std::optional<int> getPixelRadius(const Vector2i& resolution, const Vector2i& pixelCoordinates,
                                    const float robotRadius) const
  {
    const std::optional<Vector2f> robotCoordinates =
        pixelToRobotWithZ(pixelCoordinates, robotRadius);
    if (!robotCoordinates.has_value())
    {
      return {};
    }
    const Vector3f cameraCoordinates =
        camera2groundInv * Vector3f(robotCoordinates->x(), robotCoordinates->y(), 0);
    const float distance = cameraCoordinates.norm();
    if (distance <= robotRadius)
    {
      return {};
    }
    const float angle = std::asin(robotRadius / distance); // pinhole model
    return static_cast<float>(resolution.y()) * angle / (fov.y() * TO_RAD);
  }
  /**
   * calculates the y-pixel-coordinate of the horizon in the x-th column of the image
   * @param x a x-coordinate in the image
   * @return the y-coordinate of the horizon
   */
  int getHorizonHeight(int x = 0) const
  {
    int result = horizonA * x + horizonB;
    if (result < 0)
    {
      result = 0;
    }
    return result;
  }
  /**
   * @brief reset sets the camera matrix to a defined state
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["camera2ground"] << camera2ground;
    value["camera2groundInv"] << camera2groundInv;
    value["fc"] << fc;
    value["cc"] << cc;
    value["horizonA"] << horizonA;
    value["horizonB"] << horizonB;
    value["valid"] << valid;
    value["fov"] << fov;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["camera2ground"] >> camera2ground;
    value["camera2groundInv"] >> camera2groundInv;
    value["fc"] >> fc;
    value["cc"] >> cc;
    value["horizonA"] >> horizonA;
    value["horizonB"] >> horizonB;
    value["valid"] >> valid;
    value["fov"] >> fov;
  }
};
