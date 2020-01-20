#pragma once

#include <cmath>
#include <map>
#include <vector>


#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"


namespace Uni
{
  class Value;
}


// TODO Why does this exist seperatly to the keys::joints in keys.h?
/**
 * @namespace JOINTS Robot Joints
 * @brief Same namespace as keys::joints with some additional kinematic joints
 */
namespace JOINTS
{

  /// @enum JOINTS  Enumeration for Joints of whole body
  enum JOINT
  {
    HEAD_YAW,
    HEAD_PITCH,
    L_SHOULDER_PITCH,
    L_SHOULDER_ROLL,
    L_ELBOW_YAW,
    L_ELBOW_ROLL,
    L_WRIST_YAW,
    L_HAND,
    L_HIP_YAW_PITCH,
    L_HIP_ROLL,
    L_HIP_PITCH,
    L_KNEE_PITCH,
    L_ANKLE_PITCH,
    L_ANKLE_ROLL,
    R_HIP_YAW_PITCH,
    R_HIP_ROLL,
    R_HIP_PITCH,
    R_KNEE_PITCH,
    R_ANKLE_PITCH,
    R_ANKLE_ROLL,
    R_SHOULDER_PITCH,
    R_SHOULDER_ROLL,
    R_ELBOW_YAW,
    R_ELBOW_ROLL,
    R_WRIST_YAW,
    R_HAND,
    JOINTS_MAX
  };

  const std::vector<std::string> names = {
      "HeadYaw",     "HeadPitch",  "LShoulderPitch", "LShoulderRoll", "LElbowYaw", "LElbowRoll",
      "LWristYaw",   "LHand",      "LHipYawPitch",   "LHipRoll",      "LHipPitch", "LKneePitch",
      "LAnklePitch", "LAnkleRoll", "RHipYawPitch",   "RHipRoll",      "RHipPitch", "RKneePitch",
      "RAnklePitch", "RAnkleRoll", "RShoulderPitch", "RShoulderRoll", "RElbowYaw", "RElbowRoll",
      "RWristYaw",   "RHand"};

  /// @enum ADDITIONEAL_KINEMATICS Enumeration for additional relevant kinematic information (Feet)
  enum ADDITIONAL_KINEMATICS
  {
    L_FOOT = JOINTS_MAX,
    R_FOOT,
    TORSO2GROUND,
    TORSO2GROUND_IMU,
    JOINTS_ADD_MAX
  };

  extern const std::map<const enum JOINTS::JOINT, const std::string> jointsMap;
} // namespace JOINTS
/// @namespace JOINTS_L_LEG Robot Joints of Left Leg
namespace JOINTS_L_LEG
{
  /// @enum JOINTS_L_LEG Enumeration for Joints of Left Leg
  enum JOINTS_L_LEG
  {
    L_HIP_YAW_PITCH,
    L_HIP_ROLL,
    L_HIP_PITCH,
    L_KNEE_PITCH,
    L_ANKLE_PITCH,
    L_ANKLE_ROLL,
    L_LEG_MAX
  };
} // namespace JOINTS_L_LEG
/// @namespace JOINTS_R_LEG Robot Joints of Right Leg
namespace JOINTS_R_LEG
{
  /// @enum JOINTS_R_LEG Enumeration for Joints of Right Leg
  enum JOINTS_R_LEG
  {
    R_HIP_YAW_PITCH,
    R_HIP_ROLL,
    R_HIP_PITCH,
    R_KNEE_PITCH,
    R_ANKLE_PITCH,
    R_ANKLE_ROLL,
    R_LEG_MAX
  };
} // namespace JOINTS_R_LEG

/// @namespace JOINTS_R_ARM Joints of Right Arm
namespace JOINTS_R_ARM
{
  /// @enum JOINTS_R_ARM Enumeration for Joints of Right Arm
  enum JOINTS_R_ARM
  {
    R_SHOULDER_PITCH,
    R_SHOULDER_ROLL,
    R_ELBOW_YAW,
    R_ELBOW_ROLL,
    R_WRIST_YAW,
    R_HAND,
    R_ARM_MAX
  };
} // namespace JOINTS_R_ARM

/// @namespace JOINTS_L_ARM Joints of Left Arm
namespace JOINTS_L_ARM
{
  /// @enum JOINTS_L_ARM Enumeration for Joints of Left Arm
  enum JOINTS_L_ARM
  {
    L_SHOULDER_PITCH,
    L_SHOULDER_ROLL,
    L_ELBOW_YAW,
    L_ELBOW_ROLL,
    L_WRIST_YAW,
    L_HAND,
    L_ARM_MAX
  };
} // namespace JOINTS_L_ARM

/// @namespace JOINTS_HEAD Joints of Head
namespace JOINTS_HEAD
{
  /// @enum JOINTS_HEAD Enumeration for Joints of Head
  enum JOINTS_HEAD
  {
    HEAD_YAW,
    HEAD_PITCH,
    HEAD_MAX
  };
} // namespace JOINTS_HEAD

/// @namespace ELEMENTS Robot Elements
namespace ELEMENTS
{
  /// @enum ELMENTS Enumeration for robot components
  enum ELEMENT
  {
    HEAD,
    NECK,
    TORSO,
    L_SHOULDER,
    L_BICEP,
    L_ELBOW,
    L_FOREARM,
    L_HAND,
    L_PELVIS,
    L_HIP,
    L_THIGH,
    L_TIBIA,
    L_ANKLE,
    L_FOOT,
    R_PELVIS,
    R_HIP,
    R_THIGH,
    R_TIBIA,
    R_ANKLE,
    R_FOOT,
    R_SHOULDER,
    R_BICEP,
    R_ELBOW,
    R_FOREARM,
    R_HAND,
    ELEMENTS_MAX
  };

  extern const std::map<const enum ELEMENT, const std::string> elementsMap;
} // namespace ELEMENTS

namespace LINKS
{
  enum LINK
  {
    NECK_OFFSET_Z,
    SHOULDER_OFFSET_Y,
    SHOULDER_OFFSET_Z,
    UPPER_ARM_LENGTH,
    LOWER_ARM_LENGTH,
    HAND_OFFSET_X,
    HAND_OFFSET_Z,
    HIP_OFFSET_Z,
    HIP_OFFSET_Y,
    THIGH_LENGTH,
    TIBIA_LENGTH,
    FOOT_HEIGHT,
    ELBOW_OFFSET_Y,
    LINKS_MAX
  };
  extern const std::map<const enum LINK, const std::string> offsetMap;
} // namespace LINKS


namespace FSRS
{
  /// Enum for FSR
  enum FSR
  {
    L_FL,
    L_FR,
    L_RL,
    L_RR,
    R_FL,
    R_FR,
    R_RL,
    R_RR,
    FSR_MAX
  };
  extern const std::map<const enum FSR, const std::string> fsrMap;
} // namespace FSRS

namespace SPEAKERS
{
  /// Enum for speaker
  enum SPEAKER
  {
    LEFT,
    RIGHT,
    SPEAKERS_MAX
  };
} // namespace SPEAKERS

namespace MICROPHONES
{
  /// Enum for microphones
  enum MICROPHONE
  {
    FRONT,
    REAR,
    LEFT,
    RIGHT,
    MICROPHONES_MAX
  };
} // namespace MICROPHONES

namespace CAMERAS
{
  /// Enum for cameras
  enum CAMERA
  {
    TOP,
    BOTTOM,
    CAMERAS_MAX
  };
} // namespace CAMERAS

namespace INFRAREDS
{
  /// Enum for infra-red
  enum INFRARED
  {
    RIGHT,
    LEFT,
    INFRARED_MAX
  };
} // namespace INFRAREDS

namespace LEDS
{
  /// Enum for LEDs
  enum LED
  {
    RLED0,
    RLED1,
    RLED2,
    RLED3,
    RLED4,
    RLED5,
    RLED6,
    RLED7,
    LLED0,
    LLED1,
    LLED2,
    LLED3,
    LLED4,
    LLED5,
    LLED6,
    LLED7,
    LEDS_MAX
  };
} // namespace LEDS

namespace IMU
{
  /// Enum for measurement units
  enum MEASUREMENTUNIT
  {
    ACCELEROMETER,
    GYROMETER,
    IMU_MAX
  };
} // namespace IMU

namespace SONARS
{
  /// Enum for sonars
  enum SONAR
  {
    LEFT,
    RIGHT,
    SONARS_MAX
  };
} // namespace SONARS

namespace TACTILEHEADSENSORS
{
  /// Enum for tactile head sensors
  enum TACTILEHEADSENSOR
  {
    FRONT,
    MIDDLE,
    REAR,
    TACTILEHEADSENSORS_MAX
  };
} // namespace TACTILEHEADSENSORS

namespace BUTTONS
{
  /// Enum for butons
  enum BUTTON
  {
    CHEST,
    BUTTONS_MAX
  };
} // namespace BUTTONS

namespace TACTILEHANDSENSORS
{
  /// Enum for hand tacticle sensors
  enum TACTILEHANDSENSOR
  {
    LLEFT,
    LBACK,
    LRIGHT,
    RLEFT,
    RBACK,
    RRIGHT,
    TACTILEHANDSENSORS_MAX
  };
} // namespace TACTILEHANDSENSORS

namespace BUMPERS
{
  /// Enum for foot tactile sensors
  enum BUMPER
  {
    LLEFT,
    LRIGHT,
    RLEFT,
    RRIGHT,
    BUMPERS_MAX
  };
} // namespace BUMPERS

class Configuration;
struct NaoInfo;

class NaoProvider
{
public:
  static void init(Configuration& config, const NaoInfo& info);

  static float mass(const ELEMENTS::ELEMENT& element);
  static float link(const LINKS::LINK& link);
  static float minRange(const JOINTS::JOINT& joint);
  static float maxRange(const JOINTS::JOINT& joint);
  static float minRangeHeadPitch(const float& headYaw);
  static float maxRangeHeadPitch(const float& headYaw);
  static float maxRangeLAnkleRoll(const float& anklePitch);
  static float maxRangeRAnkleRoll(const float& anklePitch);
  static float minRangeLAnkleRoll(const float& anklePitch);
  static float minRangeRAnkleRoll(const float& anklePitch);
  static Vector3f com(const ELEMENTS::ELEMENT& element);
  static Vector2f fsrPosition(const FSRS::FSR& fsr);

  static float foreArmLength();
  static float maxArmLength();
  static float minArmLength();
  static float minLegLength();
  static float maxLegLength();
  static float minLElbowY();
  static float maxLElbowY();
  static float minRElbowY();
  static float maxRElbowY();


private:
  template <typename T, std::size_t POS>
  static T interpolate(
      std::vector<Eigen::Matrix<T, 3, 1>, Eigen::aligned_allocator<Eigen::Matrix<T, 3, 1>>>& src,
      const T& value);

  static void setMasses(Uni::Value& src, ELEMENTS::ELEMENT eDst);
  static void setRanges(Uni::Value& src, JOINTS::JOINT eDst);
  static void setFSRPosition(Uni::Value& src, FSRS::FSR eDst);

  static float mass_[ELEMENTS::ELEMENTS_MAX];
  static float links_[LINKS::LINKS_MAX];
  static float minRange_[JOINTS::JOINTS_MAX];
  static float maxRange_[JOINTS::JOINTS_MAX];
  static Vector3f com_[ELEMENTS::ELEMENTS_MAX];
  static Vector2f fsrPositions_[FSRS::FSR_MAX];

  static VecVector3f lookupHeadPitch_;
  static VecVector3f lookupLAnkleRoll_;
  static VecVector3f lookupRAnkleRoll_;

  static struct lengths
  {
    float foreArmLength;
    float maxArmLength;
    float minArmLength;
    float minLegLength;
    float maxLegLength;
    float minLElbowY;
    float maxLElbowY;
    float minRElbowY;
    float maxRElbowY;
  } lengths_;
};
