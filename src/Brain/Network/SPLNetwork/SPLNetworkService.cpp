#include "Brain/Network/SPLNetwork/SPLNetworkService.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/Image422.hpp"
#include <boost/asio.hpp>
#include <boost/bind/bind.hpp>
#include <chrono>
#include <cstring>
#include <stdexcept>

SPLNetworkService::SPLNetworkService(const ModuleManagerInterface& manager)
  : Module(manager)
  , useMulticast_{*this, "useMulticast"}
  , playerConfiguration_{*this}
  , cycleInfo_{*this}
  , splNetworkData_{*this}
  , ioService_{}
  , socket_{ioService_}
  , receive_{}
  , sendMessageHandle_{
        [this](const SPLStandardMessage::SPLStandardMessage& message) { sendMessage(message); }}
{
  std::uint16_t port = playerConfiguration_->port;
  boost::asio::ip::udp::endpoint localEndpoint(boost::asio::ip::udp::v4(), port);

#if defined(HULK_TARGET_SimRobot)
  if (useMulticast_())
  {
    foreignEndpoint_.address(boost::asio::ip::address_v4::from_string("239.0.0.1"));
    localEndpoint.address(foreignEndpoint_.address());
  }
  else
  {
#endif
    foreignEndpoint_.address(boost::asio::ip::address_v4::broadcast());
#if defined(HULK_TARGET_SimRobot)
  }
#endif

  foreignEndpoint_.port(port);

  socket_.open(localEndpoint.protocol());
#if defined(HULK_TARGET_SimRobot)
  if (useMulticast_())
  {
    socket_.set_option(boost::asio::socket_base::broadcast(false));
    socket_.set_option(boost::asio::ip::udp::socket::reuse_address(true));
    socket_.bind(localEndpoint);
    socket_.set_option(boost::asio::ip::multicast::hops(0));
    try
    {
      socket_.set_option(boost::asio::ip::multicast::join_group(foreignEndpoint_.address()));
    }
    catch (const boost::system::system_error& e)
    {
      Log<M_BRAIN>(LogLevel::ERROR)
          << "Multicast is not available. Network messages can not be sent.";
      throw;
    }
    socket_.set_option(boost::asio::ip::multicast::enable_loopback(true));
  }
  else
  {
#endif
    socket_.set_option(boost::asio::socket_base::broadcast(true));
    socket_.set_option(boost::asio::ip::udp::socket::reuse_address(true));
    socket_.bind(localEndpoint);
    socket_.set_option(boost::asio::ip::multicast::enable_loopback(false));
#if defined(HULK_TARGET_SimRobot)
  }
#endif

  registerForReceive();

  backgroundThread_ = std::make_shared<std::thread>([this]() {
    ioService_.run();
    Log<M_BRAIN>(LogLevel::DEBUG) << "Shutting down transceiver thread";
  });
}

SPLNetworkService::~SPLNetworkService()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

void SPLNetworkService::cycle()
{
  {
    std::lock_guard lock{mutex_};
    splNetworkData_->messages = messages_;
    messages_.clear();
  }
  splNetworkData_->sendMessage = sendMessageHandle_;
}

void SPLNetworkService::registerForReceive()
{
  socket_.async_receive_from(boost::asio::buffer(receive_), lastSenderEndpoint_,
                             boost::bind(&SPLNetworkService::onSocketReceive, this,
                                         boost::asio::placeholders::error,
                                         boost::asio::placeholders::bytes_transferred));
}

void SPLNetworkService::onSocketReceive(const boost::system::error_code& error,
                                        std::size_t bytesTransferred)
{
  const auto receivedSystemTimePoint{std::chrono::steady_clock::now()};
  if (!error)
  {
    Log<M_BRAIN>(LogLevel::DEBUG) << "Received team message";

    // Perform some data checks
    try
    {
      SPLStandardMessage::SPLStandardMessage msg;
      if (bytesTransferred < sizeof(msg) - SPL_STANDARD_MESSAGE_DATA_SIZE)
      {
        throw std::runtime_error("Message size is too small");
      }
      memcpy(&msg, receive_.data(), sizeof(msg));
      if (memcmp(msg.header, SPL_STANDARD_MESSAGE_STRUCT_HEADER, sizeof(msg.header)) != 0)
      {
        throw std::runtime_error("SPLStandardMessage header does not match");
      }

      // SPLStandardMessage Version check
      if (msg.version != SPL_STANDARD_MESSAGE_STRUCT_VERSION)
      {
        throw std::runtime_error("SPLStandardMessage does not match the implemented version");
      }
      {
        std::lock_guard lock{mutex_};
        messages_.emplace_back(msg, receivedSystemTimePoint, lastSenderEndpoint_.address());
      }
    }
    catch (const std::exception& e)
    {
      Log<M_BRAIN>(LogLevel::ERROR) << e.what();
    }
  }
  else
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Error receiving team message";
  }

  registerForReceive();
}

void SPLNetworkService::sendMessage(const SPLStandardMessage::SPLStandardMessage& message)
{
  std::shared_ptr<SPLStandardMessage::SPLStandardMessage> msg =
      std::make_shared<SPLStandardMessage::SPLStandardMessage>(message);
  socket_.async_send_to(
      boost::asio::buffer(msg.get(), static_cast<unsigned int>(
                                         msg->data - reinterpret_cast<unsigned char*>(msg.get())) +
                                         message.numOfDataBytes),
      foreignEndpoint_, [msg](const boost::system::error_code& error, std::size_t) {
        if (error)
        {
          Log<M_BRAIN>(LogLevel::ERROR) << "Error sending team message";
        }
      });
}
