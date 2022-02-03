#include "Motion/RobotKinematicsProvider/RobotKinematicsProvider.hpp"
#include "Hardware/JointUtils.hpp"

RobotKinematicsProvider::RobotKinematicsProvider(const ModuleManagerInterface& manager)
  : Module{manager}
  , bodyPose_{*this}
  , imuSensorData_{*this}
  , jointSensorData_{*this}
  , robotKinematics_{*this}
{
}

void RobotKinematicsProvider::cycle()
{
  const auto& jointAngles{jointSensorData_->getBodyAngles()};
  // determine all matrices
  robotKinematics_->matrices = forwardKinematics().getBody(jointAngles);

  const auto& imuAngles{imuSensorData_->angle};
  // From LFoot to Torso
  const auto lLegAngles = JointUtils::extractLeftLeg(jointAngles);
  const KinematicMatrix lFoot2Torso = forwardKinematics().getLFoot(lLegAngles);
  const KinematicMatrix torso2leftFoot = KinematicMatrix::rotY(imuAngles.y()) *
                                         KinematicMatrix::rotX(imuAngles.x()) *
                                         KinematicMatrix{-1.f * lFoot2Torso.posV};
  // From RFoot to Torso
  const auto rLegAngles = JointUtils::extractRightLeg(jointAngles);
  const KinematicMatrix rFoot2Torso = forwardKinematics().getRFoot(rLegAngles);
  const KinematicMatrix torso2rightFoot = KinematicMatrix::rotY(imuAngles.y()) *
                                          KinematicMatrix::rotX(imuAngles.x()) *
                                          KinematicMatrix{-1.f * rFoot2Torso.posV};

  const bool isLeftSupport = bodyPose_->supportSide > 0.f;

  const Vector3f left2RightFoot = torso2rightFoot.posV - torso2leftFoot.posV;
  const Vector3f left2RightFootXY{left2RightFoot.x(), left2RightFoot.y(), 0.f};
  const KinematicMatrix torso2ground =
      isLeftSupport ? torso2leftFoot * KinematicMatrix{left2RightFootXY / 2.f}
                    : torso2rightFoot * KinematicMatrix{left2RightFootXY / -2.f};

  constexpr float mmPerM = 1000.f;
  if (isLeftSupport)
  {
    robotKinematics_->lastGround2currentGround =
        Vector2f{lastLeft2RightFootXY_.x() / 2.f - left2RightFootXY.x() / 2.f,
                 lastLeft2RightFootXY_.y() / 2.f - left2RightFootXY.y() / 2.f} /
        mmPerM;
  }
  else // right is support
  {
    robotKinematics_->lastGround2currentGround =
        Vector2f{-lastLeft2RightFootXY_.x() / 2.f + left2RightFootXY.x() / 2.f,
                 -lastLeft2RightFootXY_.y() / 2.f + left2RightFootXY.y() / 2.f} /
        mmPerM;
  }


  robotKinematics_->torso2ground = torso2ground;
  robotKinematics_->isTorso2groundValid = bodyPose_->footContact;
  robotKinematics_->com = com().getComBody(robotKinematics_->matrices);
  lastLeft2RightFootXY_ = left2RightFootXY;
}
