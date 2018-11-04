#include "RobotProjectionProvider.hpp"

#include "Tools/Kinematics/ForwardKinematics.h"


RobotProjectionProvider::RobotProjectionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , torsoBoundaries_(*this, "torso")
  , shoulderBoundaries_(*this, "shoulder")
  , upperArmBoundaries_(*this, "upperArm")
  , lowerArm1Boundaries_(*this, "lowerArm1")
  , lowerArm2Boundaries_(*this, "lowerArm2")
  , upperLeg1Boundaries_(*this, "upperLeg1")
  , upperLeg2Boundaries_(*this, "upperLeg2")
  , footBoundaries_(*this, "foot")
  , imageData_(*this)
  , cameraMatrix_(*this)
  , jointSensorData_(*this)
  , robotProjection_(*this)
{
}

void RobotProjectionProvider::cycle()
{
  auto anglesLLeg = jointSensorData_->getLLegAngles();
  auto anglesRLeg = jointSensorData_->getRLegAngles();
  auto anglesRArm = jointSensorData_->getRArmAngles();
  auto anglesLArm = jointSensorData_->getLArmAngles();

  auto leftFoot2Torso = ForwardKinematics::getLAnkleRoll(anglesLLeg);
  auto rightFoot2Torso = ForwardKinematics::getRAnkleRoll(anglesRLeg);
  auto rightShoulderRoll2Torso = ForwardKinematics::getRShoulderRoll(anglesRArm);
  auto leftShoulderRoll2Torso = ForwardKinematics::getLShoulderRoll(anglesLArm);
  auto rightEllbowRoll2Torso = ForwardKinematics::getRElbowRoll(anglesRArm);
  auto leftEllbowRoll2Torso = ForwardKinematics::getLElbowRoll(anglesLArm);
  auto leftHipPitch2Torso = ForwardKinematics::getLHipPitch(anglesLLeg);
  auto rightHipPitch2Torso = ForwardKinematics::getRHipPitch(anglesRLeg);

  addRobotBoundaries(leftFoot2Torso, footBoundaries_(), 1);
  addRobotBoundaries(rightFoot2Torso, footBoundaries_(), -1);
  addRobotBoundaries(leftShoulderRoll2Torso, shoulderBoundaries_(), 1);
  addRobotBoundaries(rightShoulderRoll2Torso, shoulderBoundaries_(), -1);
  addRobotBoundaries(leftShoulderRoll2Torso, upperArmBoundaries_(), 1);
  addRobotBoundaries(rightShoulderRoll2Torso, upperArmBoundaries_(), -1);
  addRobotBoundaries(leftEllbowRoll2Torso, lowerArm1Boundaries_(), 1);
  addRobotBoundaries(rightEllbowRoll2Torso, lowerArm1Boundaries_(), -1);
  addRobotBoundaries(leftEllbowRoll2Torso, lowerArm2Boundaries_(), 1);
  addRobotBoundaries(rightEllbowRoll2Torso, lowerArm2Boundaries_(), -1);
  addRobotBoundaries(leftHipPitch2Torso, upperLeg1Boundaries_(), 1);
  addRobotBoundaries(rightHipPitch2Torso, upperLeg1Boundaries_(), -1);
  addRobotBoundaries(leftHipPitch2Torso, upperLeg2Boundaries_(), 1);
  addRobotBoundaries(rightHipPitch2Torso, upperLeg2Boundaries_(), -1);


  if (debug().isSubscribed(mount_ + "." + imageData_->identification))
  {
    Image draw = imageData_->image422.to444Image();
    for (auto& line : robotProjection_->lines)
    {
      Line<int> line444;
      line444.p1 = Image422::get444From422Vector(line.p1);
      line444.p2 = Image422::get444From422Vector(line.p2);
      draw.line(line444, Color::RED);
    }
    debug().sendImage(mount_ + "." + imageData_->identification, draw);
  }
}

void RobotProjectionProvider::addRobotBoundaries(const KinematicMatrix& kinMatrix,
                                                 const VecVector3f& robotPart, int sign)
{
  auto& imageDim = imageData_->image422.size;
  Vector2i pixelPoint1 = Vector2i::Zero();
  Vector2i pixelPoint2 = Vector2i::Zero();
  bool p1Valid = false, p2Valid = false;
  auto pointInTorso =
      (kinMatrix * Vector3f(robotPart[0].x(), robotPart[0].y() * sign, robotPart[0].z())) / 1000.f;
  p1Valid = cameraMatrix_->torsoToPixel(pointInTorso, pixelPoint1);
  for (auto it = std::next(robotPart.begin()); it != robotPart.end(); it++)
  {
    auto& point = *it;
    auto coord = (kinMatrix * Vector3f(point.x(), point.y() * sign, point.z())) / 1000.f;
    p2Valid = cameraMatrix_->torsoToPixel(coord, pixelPoint2);
    if (p1Valid && p2Valid)
    {
      // Must be inside the image
      if ((pixelPoint1.x() >= 0 || pixelPoint2.x() >= 0) && (pixelPoint1 != pixelPoint2) &&
          (pixelPoint1.x() < imageDim.x() || pixelPoint2.x() < imageDim.x()) &&
          (pixelPoint1.y() < imageDim.y() || pixelPoint2.y() < imageDim.y()))
      {
        robotProjection_->lines.emplace_back(pixelPoint1, pixelPoint2);
      }
    }

    p1Valid = p2Valid;
    pixelPoint1 = pixelPoint2;
  }
}
