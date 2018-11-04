#pragma once

#include "Framework/DataType.hpp"
#include "Modules/NaoProvider.h"

#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"

class CameraMatrix : public DataType<CameraMatrix>
{
public:
  /// the name of this DataType
  DataTypeName name = "CameraMatrix";
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
  void pixelToCamera(const Vector2i& pixel_coordinates, Vector3f& cameraCoordinates) const
  {
    // Note that the x axis of the camera is the the z axis of the pixel coordinate system.
    cameraCoordinates.x() = 1;
    // inverse pinhole projection
    cameraCoordinates.y() = (cc.x() - pixel_coordinates.x()) / fc.x();
    cameraCoordinates.z() = (cc.y() - pixel_coordinates.y()) / fc.y();
  }
  /**
   * @brief cameraToPixel transforms camera coordinates to pixel coordinates
   * @param cameraCoordinates the camera coordinates
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool cameraToPixel(const Vector3f& cameraCoordinates, Vector2i& pixel_coordinates) const
  {
    // A position behind the camera cannot be transformed to pixel coordinates as it does not
    // intersect the image plane.
    if (cameraCoordinates.x() <= 0.f)
    {
      return false;
    }
    // pinhole projection
    pixel_coordinates.x() = cc.x() - fc.x() * cameraCoordinates.y() / cameraCoordinates.x() +
                            0.5; // add 0.5 for mathematical rounding
    pixel_coordinates.y() = cc.y() - fc.y() * cameraCoordinates.z() / cameraCoordinates.x() + 0.5;
    return true;
  }
  /**
   * @brief pixelToRobot calculates the coordinates (on ground) in the robot coordinate system of a
   * given pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param robotCoordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool pixelToRobot(const Vector2i& pixel_coordinates, Vector2f& robotCoordinates) const
  {
    return pixelToRobot(pixel_coordinates, robotCoordinates, camera2ground);
  }
  bool pixelToRobot(const Vector2i& pixel_coordinates, Vector2f& robotCoordinates,
                    const KinematicMatrix& cam2ground) const
  {
    Vector3f cameraCoordinates;
    // apply inverse projection - This results in a ray of potential points in homogeneous
    // coordinates.
    pixelToCamera(pixel_coordinates, cameraCoordinates);
    // rotate this ray to the robot coordinate system
    cameraCoordinates = cam2ground.rotM * cameraCoordinates;
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (cameraCoordinates.z() == 0.f || std::isnan(cameraCoordinates.x()) ||
        std::isnan(cameraCoordinates.y()) || std::isnan(cameraCoordinates.z()))
    {
      return false;
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    robotCoordinates.x() =
        cam2ground.posV.x() - cam2ground.posV.z() * cameraCoordinates.x() / cameraCoordinates.z();
    robotCoordinates.y() =
        cam2ground.posV.y() - cam2ground.posV.z() * cameraCoordinates.y() / cameraCoordinates.z();
    return true;
  }
  /**
   * @brief pixelToRobotWithZ calculates the coordinates in the robot coordinate system of a given
   * pixel in an image
   * @param pixel_coordinates coordinates in the image
   * @param z a predetermined z coordinate of the plane in which to project the pixel coordinates
   * @param robotCoordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool pixelToRobotWithZ(const Vector2i& pixel_coordinates, float z,
                         Vector2f& robotCoordinates) const
  {
    Vector3f cameraCoordinates;
    // apply inverse projection - This results in a ray of potential points in homogeneous
    // coordinates.
    pixelToCamera(pixel_coordinates, cameraCoordinates);
    // rotate this ray to the robot coordinate system
    cameraCoordinates = camera2ground.rotM * cameraCoordinates;
    // If the ray is parallel to the ground, it does not intersect the ground.
    if (cameraCoordinates.z() == 0.f)
    {
      return false;
    }
    // Scale the ray so that it intersects the ground and subtract it from the camera position.
    robotCoordinates.x() = camera2ground.posV.x() - (camera2ground.posV.z() - z) *
                                                        cameraCoordinates.x() /
                                                        cameraCoordinates.z();
    robotCoordinates.y() = camera2ground.posV.y() - (camera2ground.posV.z() - z) *
                                                        cameraCoordinates.y() /
                                                        cameraCoordinates.z();
    return true;
  }
  /**
   * @brief robotToPixel calculates the pixel coordinates of a given point (on ground) in robot
   * coordinates
   * @param robotCoordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool robotToPixel(const Vector2f& robotCoordinates, Vector2i& pixel_coordinates) const
  {
    return robotToPixel(robotCoordinates, pixel_coordinates, camera2groundInv);
  }
  bool robotToPixel(const Vector2f& robotCoordinates, Vector2i& pixel_coordinates,
                    const KinematicMatrix& cam2ground_inv) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f cameraCoordinates(cam2ground_inv *
                               Vector3f(robotCoordinates.x(), robotCoordinates.y(), 0));
    // do pinhole projection
    return cameraToPixel(cameraCoordinates, pixel_coordinates);
  }

  /**
   * @brief robotWithZToPixel calculates the pixel coordinates of a given point in robot coordinates
   * @param robotCoordinates coordinates in the plane
   * @param pixel_coordinates the result is stored here
   * @return whether the transformation was successful
   */
  bool robotWithZToPixel(const Vector3f& robotCoordinates, Vector2i& pixel_coordinates) const
  {
    // calculate camera coordinates from robot coordinates
    Vector3f cameraCoordinates(camera2groundInv * robotCoordinates);
    // do pinhole projection
    return cameraToPixel(cameraCoordinates, pixel_coordinates);
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
    Vector3f cameraCoordinates(camera2torsoInv * torso_coordinates);
    // do pinhole projection
    return cameraToPixel(cameraCoordinates, pixel_coordinates);
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
  bool getPixelRadius(const Vector2i& resolution, const Vector2i& pixel_coordinates,
                      const float robot_radius, int& pixel_radius) const
  {
    Vector2f robotCoordinates;
    Vector3f cameraCoordinates;
    if (!pixelToRobotWithZ(pixel_coordinates, robot_radius, robotCoordinates))
    {
      return false;
    }
    cameraCoordinates = camera2groundInv * Vector3f(robotCoordinates.x(), robotCoordinates.y(), 0);
    float distance = cameraCoordinates.norm();
    if (distance <= robot_radius)
    {
      return false;
    }
    float angle = asin(robot_radius / distance); // pinhole model
    pixel_radius = resolution.y() * angle / (fov.y() * TO_RAD);
    return true;
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
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
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

  virtual void fromValue(const Uni::Value& value)
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
