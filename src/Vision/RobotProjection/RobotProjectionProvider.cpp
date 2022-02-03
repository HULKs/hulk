#include "Vision/RobotProjection/RobotProjectionProvider.hpp"


RobotProjectionProvider::RobotProjectionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , torsoBoundaries_(*this, "torso", [] {})
  , shoulderBoundaries_(*this, "shoulder", [] {})
  , upperArmBoundaries_(*this, "upperArm", [] {})
  , lowerArm1Boundaries_(*this, "lowerArm1", [] {})
  , lowerArm2Boundaries_(*this, "lowerArm2", [] {})
  , upperLeg1Boundaries_(*this, "upperLeg1", [] {})
  , upperLeg2Boundaries_(*this, "upperLeg2", [] {})
  , footBoundaries_(*this, "foot", [] {})
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

  auto leftFoot2Torso = forwardKinematics().getLAnkleRoll(anglesLLeg);
  auto rightFoot2Torso = forwardKinematics().getRAnkleRoll(anglesRLeg);
  auto rightShoulderRoll2Torso = forwardKinematics().getRShoulderRoll(anglesRArm);
  auto leftShoulderRoll2Torso = forwardKinematics().getLShoulderRoll(anglesLArm);
  auto rightEllbowRoll2Torso = forwardKinematics().getRElbowRoll(anglesRArm);
  auto leftEllbowRoll2Torso = forwardKinematics().getLElbowRoll(anglesLArm);
  auto leftHipPitch2Torso = forwardKinematics().getLHipPitch(anglesLLeg);
  auto rightHipPitch2Torso = forwardKinematics().getRHipPitch(anglesRLeg);

  addRobotBoundaries(KinematicMatrix(), torsoBoundaries_(), 1);
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
      draw.drawLine(line444, Color::RED);
    }
    debug().sendImage(mount_ + "." + imageData_->identification, draw);
  }
}

void RobotProjectionProvider::addRobotBoundaries(const KinematicMatrix& kinMatrix,
                                                 const VecVector3f& robotPart, int sign)
{
  const auto& imageDim = imageData_->image422.size;
  const Vector3f pointInTorso =
      (kinMatrix *
       Vector3f(robotPart[0].x(), static_cast<float>(sign) * robotPart[0].y(), robotPart[0].z())) /
      1000.f;
  std::optional<Vector2i> pixelPoint1 = cameraMatrix_->torsoToPixel(pointInTorso);
  for (auto it = std::next(robotPart.begin()); it != robotPart.end(); it++)
  {
    const auto& point = *it;
    const Vector3f coord =
        (kinMatrix * Vector3f(point.x(), static_cast<float>(sign) * point.y(), point.z())) / 1000.f;
    const std::optional<Vector2i> pixelPoint2 = cameraMatrix_->torsoToPixel(coord);
    if (pixelPoint1.has_value() && pixelPoint2.has_value())
    {
      // Must be inside the image
      if ((pixelPoint1->x() >= 0 || pixelPoint2->x() >= 0) && (pixelPoint1 != pixelPoint2) &&
          (pixelPoint1->x() < imageDim.x() || pixelPoint2->x() < imageDim.x()) &&
          (pixelPoint1->y() < imageDim.y() || pixelPoint2->y() < imageDim.y()))
      {
        robotProjection_->lines.emplace_back(pixelPoint1.value(), pixelPoint2.value());
      }
    }
    pixelPoint1 = pixelPoint2;
  }
}
