#pragma once

#include "Data/CameraMatrix.hpp"
#include "Data/ImageData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/RobotProjection.hpp"

#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"

#include <vector>

class Brain;

class RobotProjectionProvider : public Module<RobotProjectionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "RobotProjectionProvider";
  /**
   * @brief RobotProjectionProvider provides the current projection of the robot into the image
   * @param manager a reference to the module manager
   * @author Nicolas Riebesel
   */
  RobotProjectionProvider(const ModuleManagerInterface& manager);

  /**
   * @brief cycle calculates the current projection of the robot into the image
   * @author Nicolas Riebesel
   */
  void cycle();

private:
  /**
   * @brief addRobotBoundaries projects points into the image
   * @param kinMatrix Kinematic matrix for the points of the robot part
   * @param robotPart Points belonging to the robot
   * @param sign 1 if left, -1 if right
   */
  void addRobotBoundaries(const KinematicMatrix& kinMatrix, const VecVector3f& robotPart, int sign);

  const Parameter<VecVector3f> torsoBoundaries_;
  const Parameter<VecVector3f> shoulderBoundaries_;
  const Parameter<VecVector3f> upperArmBoundaries_;
  const Parameter<VecVector3f> lowerArm1Boundaries_;
  const Parameter<VecVector3f> lowerArm2Boundaries_;
  const Parameter<VecVector3f> upperLeg1Boundaries_;
  const Parameter<VecVector3f> upperLeg2Boundaries_;
  const Parameter<VecVector3f> footBoundaries_;

  /// the current image
  const Dependency<ImageData> imageData_;
  /// the current camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the current joint sensor data
  const Dependency<JointSensorData> jointSensorData_;

  Production<RobotProjection> robotProjection_;
};
