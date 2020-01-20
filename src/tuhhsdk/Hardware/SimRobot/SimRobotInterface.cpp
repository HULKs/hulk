// Parts of this file are taken from Src/Controller/SimulatedRobot.cpp from BHumanCodeRelease 2016

#include <cmath>
#include <thread>

#include <QString>
#include <QList>
#include <QVector>

#include "SimRobotNoAudio.hpp"
#include "SimRobotPortAudio.hpp"
#include "SimRobotInterface.hpp"

#include "Modules/NaoProvider.h"
#include "tuhh.hpp"


#ifdef WIN32
#include <Windows.h>

static NTSTATUS(__stdcall* NtDelayExecution)(BOOL Alertable, PLARGE_INTEGER DelayInterval) =
    (NTSTATUS(__stdcall*)(BOOL, PLARGE_INTEGER))GetProcAddress(GetModuleHandle("ntdll.dll"),
                                                               "NtDelayExecution");

static NTSTATUS(__stdcall* ZwSetTimerResolution)(IN ULONG RequestedResolution, IN BOOLEAN Set,
                                                 OUT PULONG ActualResolution) =
    (NTSTATUS(__stdcall*)(ULONG, BOOLEAN, PULONG))GetProcAddress(GetModuleHandle("ntdll.dll"),
                                                                 "ZwSetTimerResolution");

static void SleepShort(float microseconds)
{
  static bool once = true;
  if (once)
  {
    ULONG actualResolution;
    ZwSetTimerResolution(1, true, &actualResolution);
    once = false;
  }

  LARGE_INTEGER interval;
  interval.QuadPart = -1 * static_cast<int>(microseconds * 10.0f);
  NtDelayExecution(false, &interval);
}

#endif

SimRobotInterface::SimRobotInterface(SimRobot::Application& application, SimRobot::Object* robot)
  : application_(application)
  , robot_(robot)
  , otherRobots_()
  , jointAngleCommands_(keys::joints::JOINTS_MAX, 0)
  , topCamera_(Camera::TOP)
  , bottomCamera_(Camera::BOTTOM)
  , buttons_()
  , robotName_(robot_->getFullName().mid(robot_->getFullName().lastIndexOf('.') + 1).toStdString())
{

  QVector<QString> parts(1);

  // joints
  QString position(".position");
  const char* names[keys::joints::JOINTS_MAX] = {
      "HeadYaw",     "HeadPitch",  "LShoulderPitch", "LShoulderRoll", "LElbowYaw", "LElbowRoll",
      "LWristYaw",   "LHand",      "LHipYawPitch",   "LHipRoll",      "LHipPitch", "LKneePitch",
      "LAnklePitch", "LAnkleRoll", "RHipYawPitch",   "RHipRoll",      "RHipPitch", "RKneePitch",
      "RAnklePitch", "RAnkleRoll", "RShoulderPitch", "RShoulderRoll", "RElbowYaw", "RElbowRoll",
      "RWristYaw",   "RHand"};
  for (unsigned int i = 0; i < keys::joints::JOINTS_MAX; i++)
  {
    parts[0] = QString(names[i]) + position;
    jointSensors_[i] = reinterpret_cast<SimRobotCore2::SensorPort*>(
        application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort));
    jointActuators_[i] = reinterpret_cast<SimRobotCore2::ActuatorPort*>(
        application_.resolveObject(parts, robot_, SimRobotCore2::actuatorPort));
  }

  // feet
  parts[0] = "LFoot";
  leftFoot_ = reinterpret_cast<SimRobotCore2::Body*>(
      application_.resolveObject(parts, robot_, SimRobotCore2::body));
  parts[0] = "RFoot";
  rightFoot_ = reinterpret_cast<SimRobotCore2::Body*>(
      application_.resolveObject(parts, robot_, SimRobotCore2::body));

  // gyroscope
  parts[0] = "Gyroscope.angularVelocities";
  gyroscope_ = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);

  // accelerometer
  parts[0] = "Accelerometer.acceleration";
  accelerometer_ = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);

  // cameras
  parts[0] = "CameraTop.image";
  cameras_[0] = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);
  {
    const QList<int>& dimensions =
        reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[0])->getDimensions();
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    topCamera_.setSize(dimensions[0], dimensions[1]);
  }

  parts[0] = "CameraBottom.image";
  cameras_[1] = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);
  {
    const QList<int>& dimensions =
        reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[1])->getDimensions();
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    bottomCamera_.setSize(dimensions[0], dimensions[1]);
  }

  // ball
  auto balls = reinterpret_cast<SimRobotCore2::Object*>(
      application_.resolveObject("RoboCup.balls", SimRobotCore2::compound));
  if (balls)
  {
    ball_ = application_.getObjectChild(*balls, 0);
  }

  // other robots
  SimRobot::Object* group = application_.resolveObject("RoboCup.robots", SimRobotCore2::compound);
  // at this point it is ensured that the RoboCup.robots object has at least one member
  // since otherwise this code would not be called
  unsigned int totalNumberOfRobots = application_.getObjectChildCount(*group);
  assert(totalNumberOfRobots > 0);
  for (unsigned int i = 0; i < totalNumberOfRobots; i++)
  {
    auto robot = static_cast<SimRobot::Object*>(application_.getObjectChild(*group, i));
    // make sure that we don't add ourselfs
    if (robot->getFullName() != robot_->getFullName())
    {
      otherRobots_.push_back(robot);
    }
  }

  // init tuhhSDK
  tuhh_ = std::make_unique<TUHH>(*this);
}

SimRobotInterface::~SimRobotInterface()
{
  topCamera_.setShutdownRequest();
  bottomCamera_.setShutdownRequest();
  {
    std::lock_guard<std::mutex> lg(sensorDataLock_);
    shutdownRequest_ = true;
  }
  cv_.notify_one();
  imagesRendered_.notify_one();
}

void SimRobotInterface::update()
{
  // render camera images every third frame
  const bool renderImages = (counter_ % 3) == 0;
  if (renderImages)
  {
    if (topCamera_.getRequiresRenderedImage() || bottomCamera_.getRequiresRenderedImage())
    {
      std::unique_lock<std::mutex> ul(cameraDataLock_);
      if (SimRobotCamera::renderCameras({&topCamera_, &bottomCamera_}, cameras_))
      {
        ul.unlock();
        imagesRendered_.notify_one();
      }
    }
    else
    {
      {
        std::unique_lock<std::mutex> ul(cameraDataLock_);
        // if no real image is requested, set an empty one to trigger the waiting thread
        topCamera_.setImage(nullptr, TimePoint::getCurrentTime());
        bottomCamera_.setImage(nullptr, TimePoint::getCurrentTime() + std::chrono::milliseconds(1));
      }
      imagesRendered_.notify_one();
    }
  }

  {
    // This needs to be copied because it could otherwise deadlock with things.
    std::vector<float> copiedJointAngleCommands;
    {
      std::unique_lock<std::mutex> lg(jointAngleLock_);
      jointAngleCv_.wait(lg, [this] { return newJointAngles_; });
      copiedJointAngleCommands = jointAngleCommands_;
      newJointAngles_ = false;
    }
    copiedJointAngleCommands.resize(keys::joints::JOINTS_MAX, 0.f);
    std::lock_guard<std::mutex> lg(sensorDataLock_);
    for (unsigned int i = 0; i < keys::joints::JOINTS_MAX; i++)
    {
      if (jointActuators_[i] == nullptr)
      {
        sensorData_.jointSensor[i] = 0.0f;
      }
      else
      {
        sensorData_.jointSensor[i] =
            static_cast<SimRobotCore2::SensorPort*>(jointSensors_[i])->getValue().floatValue;
        reinterpret_cast<SimRobotCore2::ActuatorPort*>(jointActuators_[i])
            ->setValue(copiedJointAngleCommands[i]);
      }
      sensorData_.jointCurrent[i] = 0.0f;
      sensorData_.jointTemperature[i] = 30.0f;
      sensorData_.jointStatus[i] = 0.0f;
    }
    sensorData_.switches = buttons_;
    buttons_ = {{}};

    // Fortunately, the user menu runs in the same thread as this method.
    sensorData_.buttonCallbackList = callbacks_;
    callbacks_.clear();

    const float* floatArray =
        reinterpret_cast<SimRobotCore2::SensorPort*>(gyroscope_)->getValue().floatArray;
    sensorData_.imu[keys::sensor::IMU_GYR_X] = floatArray[0];
    sensorData_.imu[keys::sensor::IMU_GYR_Y] = floatArray[1];
    sensorData_.imu[keys::sensor::IMU_GYR_Z] = -floatArray[2];
    floatArray =
        reinterpret_cast<SimRobotCore2::SensorPort*>(accelerometer_)->getValue().floatArray;
    sensorData_.imu[keys::sensor::IMU_ACC_X] = -floatArray[0];
    sensorData_.imu[keys::sensor::IMU_ACC_Y] = floatArray[1];
    sensorData_.imu[keys::sensor::IMU_ACC_Z] = -floatArray[2];
    float position[3];
    float world2robot[3][3];
    reinterpret_cast<SimRobotCore2::Body*>(robot_)->getPose(position, world2robot);
    const float axis[2] = {world2robot[1][2], -world2robot[0][2]};
    const float axisLength = std::sqrt(axis[0] * axis[0] + axis[1] * axis[1]);
    if (axisLength == 0.0f)
    {
      sensorData_.imu[keys::sensor::IMU_ANGLE_X] = 0.0f;
      sensorData_.imu[keys::sensor::IMU_ANGLE_Y] = 0.0f;
    }
    else
    {
      const float w = std::atan2(axisLength, world2robot[2][2]) / axisLength;
      // TODO: check sign
      sensorData_.imu[keys::sensor::IMU_ANGLE_X] = axis[0] * w;
      sensorData_.imu[keys::sensor::IMU_ANGLE_Y] = axis[1] * w;
    }
    sensorData_.imu[keys::sensor::IMU_ANGLE_Z] = 0.0f;
    // Fake data:
    // the faked position of this robot:
    Pose robotPose = {position[0], position[1], std::atan2(-world2robot[1][0], world2robot[0][0])};
    fakeData_.setFakeRobotPose(robotPose);
    const auto inverseRobotPose = robotPose.inverse();
    if (ball_)
    {
      const float* absBall = reinterpret_cast<SimRobotCore2::Body*>(ball_)->getPosition();
      const Vector2f absoluteBallPosition = {absBall[0], absBall[1]};
      const Vector2f relativeBallPosition = inverseRobotPose * absoluteBallPosition;
      fakeData_.setFakeBallPosition(relativeBallPosition);
    }
    // the faked position of other robots
    // extract the postions from the otherRobots_
    VecVector2f otherRobotPositions;
    otherRobotPositions.reserve(otherRobots_.size());
    for (auto& otherRobot : otherRobots_)
    {
      float position[3];
      float world2robot[3][3];
      reinterpret_cast<SimRobotCore2::Body*>(otherRobot)->getPose(position, world2robot);
      const auto relativeOtherRobot = inverseRobotPose * Vector2f(position[0], position[1]);
      otherRobotPositions.emplace_back(relativeOtherRobot);
    }
    fakeData_.setFakeRobotPositions(otherRobotPositions);
    updateFSRs();
    sensorData_.sonar = {{}};

    sensorData_.battery[keys::sensor::BATTERY_TEMPERATURE] = 30.0f;
    sensorData_.battery[keys::sensor::BATTERY_CURRENT] = 0.0f;
    sensorData_.battery[keys::sensor::BATTERY_STATUS] = 1.0f;
    sensorData_.battery[keys::sensor::BATTERY_CHARGE] = 1.0f;

    sensorData_.time = TimePoint::getCurrentTime();
    newData_ = true;
  }
  cv_.notify_one();

  counter_++;

  if (renderImages)
  {
    const auto diff =
        std::chrono::duration_cast<std::chrono::microseconds>(Clock::now() - lastRenderCycleEnd_)
            .count();
    auto toSleep = 3 * 10000 - diff - 100; // the 100 is a little overhead
    if (toSleep > 10)
    {
#ifdef WIN32
      SleepShort(toSleep);
#else
      std::this_thread::sleep_for(std::chrono::microseconds(toSleep));
#endif
    }
    lastRenderCycleEnd_ = Clock::now();
  }
}

void SimRobotInterface::configure(Configuration& config, NaoInfo&)
{
  std::string mount = "SimRobot";
  config.mount(mount, mount + ".json", ConfigurationType::HEAD);

  auto enablePortaudio = config.get(mount, "enablePortaudio").asBool();

  if (enablePortaudio)
  {
    audio_ = std::make_unique<SimRobotPortAudio>();
  }
  else
  {
    audio_ = std::make_unique<SimRobotNoAudio>();
  }
}

void SimRobotInterface::setJointAngles(const std::vector<float>& angles)
{
  assert(angles.size() == keys::joints::JOINTS_MAX);
  {
    std::lock_guard<std::mutex> lg(jointAngleLock_);
    jointAngleCommands_ = angles;
    newJointAngles_ = true;
  }
  jointAngleCv_.notify_one();
}

void SimRobotInterface::setJointStiffnesses(const std::vector<float>&) {}

void SimRobotInterface::setLEDs(const std::vector<float>&) {}

void SimRobotInterface::setSonar(const float) {}

float SimRobotInterface::waitAndReadSensorData(NaoSensorData& data)
{
  std::unique_lock<std::mutex> lock(sensorDataLock_);
  cv_.wait(lock, [this] { return newData_ || shutdownRequest_; });
  data = sensorData_;
  newData_ = false;

  // Approximated time since last sensor reading
  return 0.012f;
}

std::string SimRobotInterface::getFileRoot()
{
  return LOCAL_FILE_ROOT;
}

std::string SimRobotInterface::getDataRoot()
{
  return getFileRoot();
}

void SimRobotInterface::getNaoInfo(Configuration& config, NaoInfo& info)
{
  info.bodyVersion = NaoVersion::V6;
  info.headVersion = NaoVersion::V6;
  info.bodyName = robotName_;
  info.headName = robotName_;

  // Export the NaoInfo to provide it in tuhhSDK.base for Export Diff functionality in MATE
  // (Not really applicable for SimRobot?)
  Uni::Value value = Uni::Value(Uni::ValueType::OBJECT);
  value << info;
  config.set("tuhhSDK.base", "NaoInfo", value);
}

CameraInterface& SimRobotInterface::getCamera(const Camera camera)
{
  return (camera == Camera::TOP) ? topCamera_ : bottomCamera_;
}

FakeDataInterface& SimRobotInterface::getFakeData()
{
  return fakeData_;
}

AudioInterface& SimRobotInterface::getAudio()
{
  return *audio_;
}

CameraInterface& SimRobotInterface::getNextCamera()
{
  std::unique_lock<std::mutex> ul(cameraDataLock_);
  std::array<SimRobotCamera*, 2> cameras = {&topCamera_, &bottomCamera_};

  imagesRendered_.wait(ul, [&]() { return SimRobotCamera::getNextCamera(cameras); });
  return *SimRobotCamera::getNextCamera(cameras);
}

Camera SimRobotInterface::getCurrentCameraType()
{
  return currentCamera_;
}

void SimRobotInterface::pressChestButton()
{
  callbacks_.push_back(CE_CHESTBUTTON_SIMPLE);
}

void SimRobotInterface::pressHeadButton(const HeadButtonType headButtonType)
{
  keys::sensor::switches switchIndex = keys::sensor::SWITCH_HEAD_FRONT;
  switch (headButtonType)
  {
    case HeadButtonType::FRONT:
      switchIndex = keys::sensor::SWITCH_HEAD_FRONT;
      break;
    case HeadButtonType::MIDDLE:
      switchIndex = keys::sensor::SWITCH_HEAD_MIDDLE;
      break;
    case HeadButtonType::REAR:
      switchIndex = keys::sensor::SWITCH_HEAD_REAR;
      break;
  }
  buttons_[switchIndex] = 1.0f;
}

const std::string& SimRobotInterface::getName() const
{
  return robotName_;
}
#undef max

void SimRobotInterface::updateFSRs()
{
  // TODO: set FSR_COP
  // the number of fsrs in each foot
  const unsigned int fsrsPerFoot = FSRS::FSR_MAX / 2;
  if (leftFoot_ && rightFoot_)
  {
    // the positions of all fsrs (in m with respect to the foot center)
    std::array<Vector2f, FSRS::FSR_MAX> fsrPositions = {
        {NaoProvider::fsrPosition(FSRS::L_FL), NaoProvider::fsrPosition(FSRS::L_FR),
         NaoProvider::fsrPosition(FSRS::L_RL), NaoProvider::fsrPosition(FSRS::L_RR),
         NaoProvider::fsrPosition(FSRS::R_FL), NaoProvider::fsrPosition(FSRS::R_FR),
         NaoProvider::fsrPosition(FSRS::R_RL), NaoProvider::fsrPosition(FSRS::R_RR)}};
    static constexpr float weight = 0.415f;
    sensorData_.fsrLeft[keys::sensor::FSR_TOTAL_WEIGHT] = 0;
    sensorData_.fsrRight[keys::sensor::FSR_TOTAL_WEIGHT] = 0;
    // figuring out how much weight we have on each foot
    for (unsigned int i = 0; i < FSRS::FSR_MAX; i++)
    {
      KinematicMatrix foot2Ground;
      // in which foot is the current fsr-index located?
      const bool isLeftFootFsr = i < fsrsPerFoot;
      // get the kinematic matrix of the foot, the current fsr-index belongs to
      getKinematicMatrix(isLeftFootFsr ? leftFoot_ : rightFoot_, foot2Ground);
      // get the position of this fsr with respect to the foot center...
      const Vector2f& fsrToFoot = fsrPositions[i];
      // ... figure out where it is in the world
      Vector3f fsrToGround = foot2Ground * Vector3f(fsrToFoot.x(), fsrToFoot.y(),
                                                    -NaoProvider::link(LINKS::FOOT_HEIGHT));
      // get a reference to the according sensorData that we want to write to
      auto& fsrData = isLeftFootFsr ? sensorData_.fsrLeft : sensorData_.fsrRight;
      // the index of the fsr within the foot
      const unsigned int fsrIndexWithinFoot = i % fsrsPerFoot;
      // reverse-engineer the fsr-reading from the amount the robot sank into the ground
      fsrData[fsrIndexWithinFoot] = std::max(0.f, -fsrToGround.z() * weight);
      // update the total weight for the accoring foot
      fsrData[keys::sensor::FSR_TOTAL_WEIGHT] += fsrData[fsrIndexWithinFoot];
    }
    // set center of pressure for completeness (never used by anyone)
    // left foot
    sensorData_.fsrLeft[keys::sensor::FSR_COP_X] = 0;
    sensorData_.fsrLeft[keys::sensor::FSR_COP_Y] = 0;
    // right foot
    sensorData_.fsrRight[keys::sensor::FSR_COP_X] = 0;
    sensorData_.fsrRight[keys::sensor::FSR_COP_Y] = 0;
  }
  else
  {
    sensorData_.fsrLeft = {{0.5, 0.5, 0.5, 0.5, 2.0, 0, 0}};
    sensorData_.fsrRight = {{0.5, 0.5, 0.5, 0.5, 2.0, 0, 0}};
  }
}

void SimRobotInterface::getKinematicMatrix(SimRobot::Object* object, KinematicMatrix& target) const
{
  float rotation[3][3];
  float position[3];
  reinterpret_cast<const SimRobotCore2::Body*>(object)->getPose(position, rotation);
  target.posV.x() = position[0];
  target.posV.y() = position[1];
  target.posV.z() = position[2];

  target.posV *= 1000.f;

  Matrix3f rot;
  rot << rotation[0][0], rotation[1][0], rotation[2][0], //
      rotation[0][1], rotation[1][1], rotation[2][1],    //
      rotation[0][2], rotation[1][2], rotation[2][2];
  target.rotM = rot;
}
