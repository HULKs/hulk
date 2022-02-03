#include "Framework/Messaging.hpp"

Sender::Sender(DataQueue& queue)
  : queue_(queue)
{
}

bool Sender::send(const Message& message)
{
  return queue_.push(message);
}

const std::vector<std::type_index>& Sender::getRequested() const
{
  return requested_;
}

void Sender::produce(const std::type_index& type)
{
  produced_.push_back(type);
}

Receiver::Receiver(Sender& sender)
  : sender_(sender)
{
}

bool Receiver::receive(Message& message)
{
  return sender_.queue_.pop(message);
}

void Receiver::request(const std::type_index& type)
{
  sender_.requested_.push_back(type);
}

const std::vector<std::type_index> Receiver::getProduced() const
{
  return sender_.produced_;
}

DuplexChannel::DuplexChannel()
  : a2b_()
  , b2a_()
  , a2bSender_(a2b_)
  , a2bReceiver_(a2bSender_)
  , b2aSender_(b2a_)
  , b2aReceiver_(b2aSender_)
{
}

Sender& DuplexChannel::getA2BSender()
{
  return a2bSender_;
}

Receiver& DuplexChannel::getA2BReceiver()
{
  return a2bReceiver_;
}

Sender& DuplexChannel::getB2ASender()
{
  return b2aSender_;
}

Receiver& DuplexChannel::getB2AReceiver()
{
  return b2aReceiver_;
}
