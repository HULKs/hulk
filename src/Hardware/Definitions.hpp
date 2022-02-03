#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/EnumArray.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include <array>

enum class CameraPosition
{
  TOP,   ///< value for top camera
  BOTTOM ///< value for bottom camera
};

enum class Joints
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
  MAX
};


template <typename T>
using JointsArray = EnumArray<T, Joints, static_cast<std::size_t>(Joints::MAX)>;

static constexpr JointsArray<const char*> JOINT_NAMES = {
    {"HeadYaw",     "HeadPitch",  "LShoulderPitch", "LShoulderRoll", "LElbowYaw", "LElbowRoll",
     "LWristYaw",   "LHand",      "LHipYawPitch",   "LHipRoll",      "LHipPitch", "LKneePitch",
     "LAnklePitch", "LAnkleRoll", "RHipYawPitch",   "RHipRoll",      "RHipPitch", "RKneePitch",
     "RAnklePitch", "RAnkleRoll", "RShoulderPitch", "RShoulderRoll", "RElbowYaw", "RElbowRoll",
     "RWristYaw",   "RHand"}};


enum class JointsLeg
{
  HIP_YAW_PITCH,
  HIP_ROLL,
  HIP_PITCH,
  KNEE_PITCH,
  ANKLE_PITCH,
  ANKLE_ROLL,
  MAX
};

template <typename T>
using JointsLegArray = EnumArray<T, JointsLeg, static_cast<std::size_t>(JointsLeg::MAX)>;

enum class JointsArm
{
  SHOULDER_PITCH,
  SHOULDER_ROLL,
  ELBOW_YAW,
  ELBOW_ROLL,
  WRIST_YAW,
  HAND,
  MAX
};

template <typename T>
using JointsArmArray = EnumArray<T, JointsArm, static_cast<std::size_t>(JointsArm::MAX)>;

enum class JointsHead
{
  YAW,
  PITCH,
  MAX
};

template <typename T>
using JointsHeadArray = EnumArray<T, JointsHead, static_cast<std::size_t>(JointsHead::MAX)>;

enum class Elements
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
  MAX
};

enum class Links
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
  MAX
};

enum class Speakers
{
  LEFT,
  RIGHT,
  MAX
};
template <typename T>
using SpeakersArray = EnumArray<T, Speakers, static_cast<std::size_t>(Speakers::MAX)>;

enum class Microphones
{
  FRONT,
  REAR,
  LEFT,
  RIGHT,
  MAX
};
template <typename T>
using MicrophonesArray = EnumArray<T, Microphones, static_cast<std::size_t>(Microphones::MAX)>;

enum class Infrareds
{
  RIGHT,
  LEFT,
  MAX
};

template <typename T>
using InfraredsArray = EnumArray<T, Infrareds, static_cast<std::size_t>(Infrareds::MAX)>;

enum class FSRs
{
  L_FRONT_LEFT,
  L_FRONT_RIGHT,
  L_REAR_LEFT,
  L_REAR_RIGHT,
  R_FRONT_LEFT,
  R_FRONT_RIGHT,
  R_REAR_LEFT,
  R_REAR_RIGHT,
  MAX
};

template <typename T>
using FSRsArray = EnumArray<T, FSRs, static_cast<std::size_t>(FSRs::MAX)>;

enum class Sonars
{
  LEFT,
  RIGHT,
  MAX
};

template <typename T>
using SonarsArray = EnumArray<T, Sonars, static_cast<std::size_t>(Sonars::MAX)>;

enum class Cameras
{
  TOP,
  BOTTOM,
  MAX
};

template <typename T>
using CamerasArray = EnumArray<T, Cameras, static_cast<std::size_t>(Cameras::MAX)>;

enum class Switches
{
  CHEST_BUTTON,
  HEAD_FRONT,
  HEAD_MIDDLE,
  HEAD_REAR,
  L_FOOT_LEFT,
  L_FOOT_RIGHT,
  L_HAND_BACK,
  L_HAND_LEFT,
  L_HAND_RIGHT,
  R_FOOT_LEFT,
  R_FOOT_RIGHT,
  R_HAND_BACK,
  R_HAND_LEFT,
  R_HAND_RIGHT,
  MAX
};

template <typename T>
using SwitchesArray = EnumArray<T, Switches, static_cast<std::size_t>(Switches::MAX)>;

enum class BodySwitches
{
  CHEST_BUTTON,
  L_FOOT_LEFT,
  L_FOOT_RIGHT,
  L_HAND_BACK,
  L_HAND_LEFT,
  L_HAND_RIGHT,
  R_FOOT_LEFT,
  R_FOOT_RIGHT,
  R_HAND_BACK,
  R_HAND_LEFT,
  R_HAND_RIGHT,
  MAX
};

template <typename T>
using BodySwitchesArray = EnumArray<T, BodySwitches, static_cast<std::size_t>(BodySwitches::MAX)>;

enum class HeadSwitches
{
  HEAD_FRONT,
  HEAD_MIDDLE,
  HEAD_REAR,
  MAX,
};

template <typename T>
using HeadSwitchesArray = EnumArray<T, HeadSwitches, static_cast<std::size_t>(HeadSwitches::MAX)>;

enum class LEDs
{
  CHEST,
  L_EAR,
  R_EAR,
  L_EYE,
  R_EYE,
  SKULL,
  L_FOOT,
  R_FOOT,
  MAX
};

template <typename T>
using LEDsArray = EnumArray<T, LEDs, static_cast<std::size_t>(LEDs::MAX)>;

enum class BodyLEDs
{
  CHEST,
  L_FOOT,
  R_FOOT,
  MAX
};

template <typename T>
using BodyLEDsArray = EnumArray<T, BodyLEDs, static_cast<std::size_t>(BodyLEDs::MAX)>;

enum class HeadLEDs
{
  L_EAR,
  R_EAR,
  L_EYE,
  R_EYE,
  SKULL,
  MAX
};

template <typename T>
using HeadLEDsArray = EnumArray<T, HeadLEDs, static_cast<std::size_t>(HeadLEDs::MAX)>;

struct FSRInfo : public Uni::To, public Uni::From
{
  float frontLeft = 0.f;
  float frontRight = 0.f;
  float rearLeft = 0.f;
  float rearRight = 0.f;

  void toValue(Uni::Value& value) const override;
  void fromValue(const Uni::Value& value) override;
};

struct IMU : public Uni::To, public Uni::From
{
  Vector3f gyroscope;
  Vector2f angle;
  Vector3f accelerometer;

  void toValue(Uni::Value& value) const override;
  void fromValue(const Uni::Value& value) override;
};

struct SonarInfo : public Uni::To, public Uni::From
{
  float leftSensor = 0.f;
  float rightSensor = 0.f;

  void toValue(Uni::Value& value) const override;
  void fromValue(const Uni::Value& value) override;
};

struct SwitchInfo : public Uni::To, public Uni::From
{
  bool isChestButtonPressed{false};
  bool isHeadFrontPressed{false};
  bool isHeadMiddlePressed{false};
  bool isHeadRearPressed{false};
  bool isLeftFootLeftPressed{false};
  bool isLeftFootRightPressed{false};
  bool isLeftHandBackPressed{false};
  bool isLeftHandLeftPressed{false};
  bool isLeftHandRightPressed{false};
  bool isRightFootLeftPressed{false};
  bool isRightFootRightPressed{false};
  bool isRightHandBackPressed{false};
  bool isRightHandLeftPressed{false};
  bool isRightHandRightPressed{false};

  void toValue(Uni::Value& value) const override;
  void fromValue(const Uni::Value& value) override;
};

namespace Led
{
  struct Color : public Uni::To, public Uni::From
  {
    Color() = default;
    Color(float red, float green, float blue);
    explicit Color(std::uint32_t rgb);

    std::uint32_t toRGB() const;
    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;

    float red{0.f};
    float green{0.f};
    float blue{0.f};
  };

  struct Chest : public Uni::To, public Uni::From
  {
    Color color;

    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;
  };

  struct Ear : public Uni::To, public Uni::From
  {
    float intensityAt0{0.f};
    float intensityAt36{0.f};
    float intensityAt72{0.f};
    float intensityAt108{0.f};
    float intensityAt144{0.f};
    float intensityAt180{0.f};
    float intensityAt216{0.f};
    float intensityAt252{0.f};
    float intensityAt288{0.f};
    float intensityAt324{0.f};

    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;
  };

  struct Eye : public Uni::To, public Uni::From
  {
    Color colorAt0;
    Color colorAt45;
    Color colorAt90;
    Color colorAt135;
    Color colorAt180;
    Color colorAt225;
    Color colorAt270;
    Color colorAt315;

    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;
  };

  struct Foot : public Uni::To, public Uni::From
  {
    Color color;

    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;
  };
} // namespace Led
