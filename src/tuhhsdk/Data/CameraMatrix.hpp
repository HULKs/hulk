#pragma once

#include "Framework/DataType.hpp"
#include "Modules/NaoProvider.h"

#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"

class CameraMatrix : public DataType<CameraMatrix>
{
public:
  /// a transformation matrix that describes the camera to the robot coordinate system - updated every cycle
  KinematicMatrix camera2ground;
  /// the inverse camera2ground for performance reasons - updated every cycle
  KinematicMatrix camera2ground_inv;
  /// a transformation matrix that describes the camera to the robot torso coordinate system - updated every cycle
  KinematicMatrix camera2torso;
  /// the inverse camera2torso for performance reasons - updated every cycle
  KinematicMatrix camera2torso_inv;
  /// the focal length with compensation for pixel size
  Vector2f fc;
  /// the optical center in pixel coordinates
  Vector2f cc;
  /// coefficients for a linear equation through the horizon: y(x) = a*x + b
  float horizon_a, horizon_b;
  /// whether the camera matrix is valid
  bool valid;
  /**
   * @brief pixelToCamera transforms pixel coordinates to camera coordinates using a pinhole camera model
   * @param pixel_coordinates the pixel coordinates
   * @param camera_coordinates the result is stored here
   */
  void pixelToCamera(const Vector2i& pixel_coordinates, Vector3f& camera_coordinates) const
  {
    // Note that the x axis of the camera is the the z axis of the pixel coordinate system.
    camera_coordinates.x() = 1;
    // inverse pinhole projection
    camera_coordinates.y() = (cc.x() - pixel_coordinates.x()) / fc.x();
    camera_coordinates.z() = (cc.y() - pixel_coordinates.y()) / fc.y();
  }
  /**
   * @brief cameraToPixel transforms camera coordinates to pixel coordinates
   * @param camera_coordinates the camera coordinates
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool cameraToPixel(const Vector3f& camera_coordinates, Vector2i& pixel_coordinates) const
  {
    // A position behind the camera cannot be transformed to pixel coordinates as it does not intersect the image plane.
    if (camera_coordinates.x() <= 0.f)
    {
      return false;
    }
    // pinhole projection
    pixel_coordinates.x() = cc.x() - fc.x() * camera_coordinates.y() / camera_coordinates.x() + 0.5; // add 0.5 for mathematical rounding
    pixel_coordinates.y() = cc.y() - fc.y() * camera_coordinates.z() / camera_coordinates.x() + 0.5;
    return true;
  }
  /**
   * @brief pixelToRobot calculates the coordinates (on ground) in the robot coordinate system of a given pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param robot_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool pixelToRobot(const Vector2i& pixel_coordinates, Vector2f& robot_coordinates) const
  {
    Vector3f camera_coordinates;
    // apply inverse projection - This results in a ray of potential points in homogeneous coordinates.
    pixelToCamera(pixel_coordinates, camera_coordinates);
    // rotate this ray to the robot coordinate system
    camera_coordinates = camera2ground.rotM * camera_coordinates;
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (camera_coordinates.z() == 0.f || std::isnan(camera_coordinates.x()) || std::isnan(camera_coordinates.y()) || std::isnan(camera_coordinates.z()))
    {
      return false;
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    robot_coordinates.x() = camera2ground.posV.x() - camera2ground.posV.z() * camera_coordinates.x() / camera_coordinates.z();
    robot_coordinates.y() = camera2ground.posV.y() - camera2ground.posV.z() * camera_coordinates.y() / camera_coordinates.z();
    return true;
  }
  /**
   * @brief pixelToRobotWithZ calculates the coordinates in the robot coordinate system of a given pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param z a predetermined z coordinate of the plane in which to project the pixel coordinates
   * @param robot_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool pixelToRobotWithZ(const Vector2i& pixel_coordinates, float z, Vector2f& robot_coordinates) const
  {
    Vector3f camera_coordinates;
    // apply inverse projection - This results in a ray of potential points in homogeneous coordinates.
    pixelToCamera(pixel_coordinates, camera_coordinates);
    // rotate this ray to the robot coordinate system
    camera_coordinates = camera2ground.rotM * camera_coordinates;
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (camera_coordinates.z() == 0.f)
    {
      return false;
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    robot_coordinates.x() = camera2ground.posV.x() - (camera2ground.posV.z() - z) * camera_coordinates.x() / camera_coordinates.z();
    robot_coordinates.y() = camera2ground.posV.y() - (camera2ground.posV.z() - z) * camera_coordinates.y() / camera_coordinates.z();
    return true;
  }
  /**
   * @brief robotToPixel calculates the pixel coordinates of a given point (on ground) in robot coordinates
   * @param robot_coordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool robotToPixel(const Vector2f& robot_coordinates, Vector2i& pixel_coordinates) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f camera_coordinates(camera2ground_inv * Vector3f(robot_coordinates.x(), robot_coordinates.y(), 0));
    // do pinhole projection
    return cameraToPixel(camera_coordinates, pixel_coordinates);
  }

  /**
   * @brief robotWithZToPixel calculates the pixel coordinates of a given point in robot coordinates
   * @param robot_coordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool robotWithZToPixel(const Vector3f& robot_coordinates, Vector2i& pixel_coordinates) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f camera_coordinates(camera2ground_inv * robot_coordinates);
    // do pinhole projection
    return cameraToPixel(camera_coordinates, pixel_coordinates);
  }
  /**
   * @brief robotToPixel calculates the pixel coordinates of a given point in robot coordinates
   * @param torso_coordinates world coordinates
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool torsoToPixel(const Vector3f& torso_coordinates, Vector2i& pixel_coordinates) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f camera_coordinates(camera2torso_inv * torso_coordinates);
    // do pinhole projection
    return cameraToPixel(camera_coordinates, pixel_coordinates);
  }
  /**
   * @brief getPixelRadius finds out the radius in pixel coordinates that a circle at some point in pixel coordinates would have
   * @param resolution the current camera resolution in px
   * @param pixel_coordinates a point in pixel coordinates where the size of an object should be calculated
   * @param robot_radius the known radius in robot coordinates
   * @param pixel_radius the estimates radius in pixel coordinates
   * @return true iff the projection was actually possible
   */
  bool getPixelRadius(const Vector2i& resolution, const Vector2i& pixel_coordinates, const float robot_radius, int& pixel_radius) const
  {
    Vector2f robot_coordinates;
    Vector3f camera_coordinates;
    if (!pixelToRobotWithZ(pixel_coordinates, robot_radius, robot_coordinates))
    {
      return false;
    }
    camera_coordinates = camera2ground_inv * Vector3f(robot_coordinates.x(), robot_coordinates.y(), 0);
    float distance = camera_coordinates.norm();
    if (distance <= robot_radius)
    {
      return false;
    }
    float angle = asin(robot_radius / distance);              // pinhole model
    pixel_radius = resolution.x() * angle / (60.97 * TO_RAD); // 60.97 is the horizontal field of view
    return true;
  }
  /**
   * calculates the y-pixel-coordinate of the horizon in the x-th column of the image
   * @param x a x-coordinate in the image
   * @return the y-coordinate of the horizon
   */
  int getHorizonHeight(int x = 0) const
  {
    int result = horizon_a * x + horizon_b;
    if (result < 0)
    {
      result = 0;
    }
    return result;
  }
  /**
   * @brief reset sets the camera matrix to a defined state
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["camera2ground"] << camera2ground;
    value["camera2ground_inv"] << camera2ground_inv;
    value["fc"] << fc;
    value["cc"] << cc;
    value["horizon_a"] << horizon_a;
    value["horizon_b"] << horizon_b;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["camera2ground"] >> camera2ground;
    value["camera2ground_inv"] >> camera2ground_inv;
    value["fc"] >> fc;
    value["cc"] >> cc;
    value["horizon_a"] >> horizon_a;
    value["horizon_b"] >> horizon_b;
    value["valid"] >> valid;
  }
};
