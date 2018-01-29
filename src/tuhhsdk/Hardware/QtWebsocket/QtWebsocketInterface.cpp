#include <QCoreApplication>

#include "Definitions/keys.h"
#include "Modules/NaoProvider.h"

#include "QtWebsocketInterface.hpp"

#include "print.h"
#include <QDebug>
#include <QWebSocket>

////////////////////////////

TcpServer::TcpServer(QtWebsocketInterface* robotInterface, QObject* parent)
  : QObject(parent)
{
  this->robotInterface = robotInterface;
  server = new QWebSocketServer("NaoQtWebsocketInterface", QWebSocketServer::NonSecureMode, this);
  connect(server, SIGNAL(newConnection()), this, SLOT(acceptConnection()));
  connect(server, SIGNAL(newConnection()), this, SLOT(acceptConnection()));
  if (server->listen(QHostAddress::Any, 8080))
  {
    Log(LogLevel::ERROR) << "Echoserver listening on port" << 8080;
  }
}

TcpServer::~TcpServer()
{
  server->close();
}

void TcpServer::acceptConnection()
{
  QWebSocket* client = server->nextPendingConnection();
  clients << client;
  connect(client, &QWebSocket::textMessageReceived, this, &TcpServer::processTextMessage);
  connect(client, &QWebSocket::binaryMessageReceived, this, &TcpServer::processBinaryMessage);
  connect(client, &QWebSocket::disconnected, this, &TcpServer::socketDisconnected);
}

void TcpServer::processTextMessage(QString message)
{
  if (message == "Ping")
  {
    QWebSocket* client = qobject_cast<QWebSocket*>(sender());
    if (client)
      client->sendTextMessage("Pong");
    return;
  }

  QStringList list = message.split(";");

  if (list.length() > 1)
  {
    if (list[0] == "Accel" && list.length() == 4)
    {
      float accelX = list[2].toFloat();
      float accelY = list[1].toFloat();
      float accelZ = list[3].toFloat();

      robotInterface->updateAccelData(accelX, accelY, accelZ);
    }
    else if (list[0] == "Gyro" && list.length() == 4)
    {
      float gyroX = list[2].toFloat();
      float gyroY = list[1].toFloat();
      float gyroZ = list[3].toFloat();

      robotInterface->updateGyroData(gyroX, gyroY, gyroZ);
    }
    else if (list[0] == "SetYaw" && list.length() == 2)
    {
      // not possible anymore
    }
  }
}

void TcpServer::processBinaryMessage(QByteArray message)
{
  Q_UNUSED(message)
}

void TcpServer::socketDisconnected()
{
  QWebSocket* pClient = qobject_cast<QWebSocket*>(sender());
  if (pClient)
  {
    clients.removeAll(pClient);
    pClient->deleteLater();
  }
}

////////////////////////////

QtWebsocketInterface::QtWebsocketInterface(int argc, char** argv)
{
  qtMainThread = std::thread([&, argc, argv]() {
    int argcCopy = argc;
    QCoreApplication a(argcCopy, argv);

    TcpServer socket(this);

    a.exec();
  });
}

void QtWebsocketInterface::configure(Configuration&) {}

void QtWebsocketInterface::setJointAngles(const std::vector<float>&) {}

void QtWebsocketInterface::setJointStiffnesses(const std::vector<float>&) {}

void QtWebsocketInterface::setLEDs(const std::vector<float>&) {}

void QtWebsocketInterface::setSonar(const float) {}

void QtWebsocketInterface::waitAndReadSensorData(NaoSensorData& data)
{
  // std::this_thread::sleep_for(std::chrono::duration<int, std::milli>(10));
  bool isDataAvailable;
  {
    std::lock_guard<std::mutex> lock(updateLock);
    isDataAvailable = (accelerometer_.size() > 0) || (gyroscope_.size() > 0);
  }
  if (!isDataAvailable)
  {
    std::unique_lock<std::mutex> lck(sensorUpdateMutex);
    sensorUpdateAvailable.wait(lck);
  }
  // dummy sensor fusion test
  {
    std::lock_guard<std::mutex> lock(updateLock);
    if (accelerometer_.size() > 0)
    {
      Vector3f accelData = accelerometer_.front();
      accelerometer_.pop();
      data.imu[keys::sensor::IMU_ACC_X] = accelData.x();
      data.imu[keys::sensor::IMU_ACC_Y] = accelData.y();
      data.imu[keys::sensor::IMU_ACC_Z] = accelData.z();
    }
    else
    {
      data.imu[keys::sensor::IMU_ACC_X] = 0.;
      data.imu[keys::sensor::IMU_ACC_Y] = 0.;
      data.imu[keys::sensor::IMU_ACC_Z] = 0.;
    }

    if (gyroscope_.size() > 0)
    {
      Vector3f gyroData = gyroscope_.front();
      gyroscope_.pop();
      data.imu[keys::sensor::IMU_GYR_X] = gyroData.x();
      data.imu[keys::sensor::IMU_GYR_Y] = gyroData.y();
      data.imu[keys::sensor::IMU_GYR_Z] = gyroData.z();
    }
    else
    {
      data.imu[keys::sensor::IMU_GYR_X] = 0.;
      data.imu[keys::sensor::IMU_GYR_Y] = 0.;
      data.imu[keys::sensor::IMU_GYR_Z] = 0.;
    }
  }
}

std::string QtWebsocketInterface::getFileRoot()
{
  return LOCAL_FILE_ROOT;
}

void QtWebsocketInterface::getNaoInfo(Configuration&, NaoInfo& info)
{
  info.bodyVersion = NaoVersion::V3_3;
  info.headVersion = NaoVersion::V4;
  info.bodyName = "webots";
  info.headName = "webots";
}

CameraInterface& QtWebsocketInterface::getCamera(const Camera camera)
{
  return (camera == Camera::TOP) ? topCamera : bottomCamera;
}

CameraInterface& QtWebsocketInterface::getCurrentCamera()
{
  // The selection which camera to use, was determined with a fair dice roll.
  return topCamera;
}

void QtWebsocketInterface::updateAccelData(float x, float y, float z)
{
  std::lock_guard<std::mutex> lock(updateLock);
  accelerometer_.push(Vector3<float>(x, y, z));

  std::unique_lock<std::mutex> lck(sensorUpdateMutex);
  sensorUpdateAvailable.notify_all();
}

void QtWebsocketInterface::updateGyroData(float x, float y, float z)
{
  std::lock_guard<std::mutex> lock(updateLock);
  gyroscope_.push(Vector3<float>(x, y, z));

  std::unique_lock<std::mutex> lck(sensorUpdateMutex);
  sensorUpdateAvailable.notify_all();
}
