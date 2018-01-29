// Parts of this file are taken from Src/Controller/SimulatedRobot.cpp from BHumanCodeRelease 2016

#include <cmath>
#include <thread>

#include <QString>
#include <QVector>

#include "SimRobotInterface.hpp"

#include "tuhh.hpp"


#ifdef WIN32
#include <Windows.h>

static NTSTATUS(__stdcall *NtDelayExecution)(BOOL Alertable, PLARGE_INTEGER DelayInterval) = (NTSTATUS(__stdcall*)(BOOL, PLARGE_INTEGER)) GetProcAddress(GetModuleHandle("ntdll.dll"), "NtDelayExecution");

static NTSTATUS(__stdcall *ZwSetTimerResolution)(IN ULONG RequestedResolution, IN BOOLEAN Set, OUT PULONG ActualResolution) = (NTSTATUS(__stdcall*)(ULONG, BOOLEAN, PULONG)) GetProcAddress(GetModuleHandle("ntdll.dll"), "ZwSetTimerResolution");

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
  , jointAngleCommands_(keys::joints::JOINTS_MAX, 0)
  , topCamera_(Camera::TOP)
  , bottomCamera_(Camera::BOTTOM)
  , buttons_()
  , robotName_(robot_->getFullName().mid(robot_->getFullName().lastIndexOf('.') + 1).toStdString())
{

  QVector<QString> parts(1);

  // joints
  QString position(".position");
  const char* names[keys::joints::JOINTS_MAX] = { "HeadYaw", "HeadPitch",
    "LShoulderPitch", "LShoulderRoll", "LElbowYaw", "LElbowRoll", "LWristYaw", "LHand",
    "LHipYawPitch", "LHipRoll", "LHipPitch", "LKneePitch", "LAnklePitch", "LAnkleRoll",
    "RHipYawPitch", "RHipRoll", "RHipPitch", "RKneePitch", "RAnklePitch", "RAnkleRoll",
    "RShoulderPitch", "RShoulderRoll", "RElbowYaw", "RElbowRoll", "RWristYaw", "RHand" };
  for (unsigned int i = 0; i < keys::joints::JOINTS_MAX; i++)
  {
    parts[0] = QString(names[i]) + position;
    jointSensors_[i] = reinterpret_cast<SimRobotCore2::SensorPort*>(application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort));
    jointActuators_[i] = reinterpret_cast<SimRobotCore2::ActuatorPort*>(application_.resolveObject(parts, robot_, SimRobotCore2::actuatorPort));
  }

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
    const QList<int>& dimensions = reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[0])->getDimensions();
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    topCamera_.setSize(dimensions[0], dimensions[1]);
  }

  parts[0] = "CameraBottom.image";
  cameras_[1] = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);
  {
    const QList<int>& dimensions = reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[1])->getDimensions();
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    bottomCamera_.setSize(dimensions[0], dimensions[1]);
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
}

void SimRobotInterface::update()
{
  // render camera images every third frame
  const bool renderImages = (counter_ % 3) == 0;
  if (renderImages)
  {
    reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[0])->renderCameraImages(reinterpret_cast<SimRobotCore2::SensorPort**>(&cameras_), 2);
    topCamera_.setImage(reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[0])->getValue().byteArray, TimePoint::getCurrentTime());
    bottomCamera_.setImage(reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[1])->getValue().byteArray, TimePoint::getCurrentTime() + std::chrono::milliseconds(1));
  }

  {
    // This needs to be copied because it could otherwise deadlock with things.
    std::vector<float> copiedJointAngleCommands;
    {
      std::unique_lock<std::mutex> lg(jointAngleLock_);
      jointAngleCv_.wait(lg, [this]{return newJointAngles_;});
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
        sensorData_.jointSensor[i] = static_cast<SimRobotCore2::SensorPort*>(jointSensors_[i])->getValue().floatValue;
        reinterpret_cast<SimRobotCore2::ActuatorPort*>(jointActuators_[i])->setValue(copiedJointAngleCommands[i]);
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

    const float* floatArray = reinterpret_cast<SimRobotCore2::SensorPort*>(gyroscope_)->getValue().floatArray;
    sensorData_.imu[keys::sensor::IMU_GYR_X] = floatArray[0];
    sensorData_.imu[keys::sensor::IMU_GYR_Y] = floatArray[1];
    sensorData_.imu[keys::sensor::IMU_GYR_Z] = -floatArray[2];
    floatArray = reinterpret_cast<SimRobotCore2::SensorPort*>(accelerometer_)->getValue().floatArray;
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

    // TODO
    sensorData_.fsrLeft = {{0.5, 0.5, 0.5, 0.5, 2.0, 0, 0}};
    sensorData_.fsrRight = {{0.5, 0.5, 0.5, 0.5, 2.0, 0, 0}};
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
    const auto diff = std::chrono::duration_cast<std::chrono::microseconds>(Clock::now() - lastRenderCycleEnd_).count();
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

void SimRobotInterface::configure(Configuration&)
{
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

void SimRobotInterface::setJointStiffnesses(const std::vector<float>&)
{
}

void SimRobotInterface::setLEDs(const std::vector<float>&)
{
}

void SimRobotInterface::setSonar(const float)
{
}

void SimRobotInterface::waitAndReadSensorData(NaoSensorData& data)
{
  std::unique_lock<std::mutex> lock(sensorDataLock_);
  cv_.wait(lock, [this]{return newData_ || shutdownRequest_;});
  data = sensorData_;
  newData_ = false;
}

std::string SimRobotInterface::getFileRoot()
{
  return LOCAL_FILE_ROOT;
}

void SimRobotInterface::getNaoInfo(Configuration&, NaoInfo& info)
{
  info.bodyVersion = NaoVersion::V4;
  info.headVersion = NaoVersion::V4;
  info.bodyName = robotName_;
  info.headName = robotName_;
}

CameraInterface& SimRobotInterface::getCamera(const Camera camera)
{
  return (camera == Camera::TOP) ? topCamera_ : bottomCamera_;
}

AudioInterface& SimRobotInterface::getAudio()
{
  return audio_;
}

CameraInterface& SimRobotInterface::getCurrentCamera()
{
  if (currentCamera_ == Camera::TOP)
  {
    currentCamera_ = Camera::BOTTOM;
    return topCamera_;
  }
  else
  {
    currentCamera_ = Camera::TOP;
    return bottomCamera_;
  }
}

void SimRobotInterface::pressChestButton()
{
  callbacks_.push_back(CE_CHESTBUTTON_SIMPLE);
}

void SimRobotInterface::pressHeadButton(const HeadButtonType headButtonType)
{
  keys::sensor::switches switchIndex = keys::sensor::SWITCH_HEAD_FRONT;
  switch (headButtonType) {
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
