from sqlalchemy import Integer, String, Null, BigInteger, Index
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


class Base(DeclarativeBase):
    pass


class Services(Base):
    __tablename__ = "services"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    service: Mapped[str] = mapped_column(String(30), default=Null)
    pattern: Mapped[str] = mapped_column(String(128), default=Null)

    def __repr__(self) -> str:
        return f"Services(id={self.id!r}, service={self.service!r}, pattern={self.pattern!r})"

    def _asdict(self):
        return {
            "id": self.id,
            "service": self.service,
            "pattern": self.pattern,
        }


class Nodes(Base):
    __tablename__ = "nodes"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    service: Mapped[int] = mapped_column(Integer, nullable=False)
    node: Mapped[str] = mapped_column(String(64), nullable=False)
    available: Mapped[int] = mapped_column(Integer, nullable=False)
    current_load: Mapped[int] = mapped_column(Integer, nullable=False)
    capacity: Mapped[int] = mapped_column(Integer, nullable=False)
    downed: Mapped[int] = mapped_column(Integer, nullable=False)
    backoff: Mapped[int] = mapped_column(Integer, nullable=False)

    unique_idx = Index(service, node)

    def __repr__(self) -> str:
        return f"Nodes(id={self.id!r}, service={self.service!r}, node={self.node!r}, available={self.available!r}, current_load={self.current_load!r}, capacity={self.capacity!r}, downed={self.downed!r}, backoff={self.backoff!r})"

    def _asdict(self):
        return {
            "id": self.id,
            "service": self.service,
            "node": self.node,
            "available": self.available,
            "current_load": self.current_load,
            "capacity": self.capacity,
            "downed": self.downed,
            "backoff": self.backoff,
        }


class Users(Base):
    __tablename__ = "users"

    uid: Mapped[int] = mapped_column(Integer, primary_key=True)
    service: Mapped[int] = mapped_column(Integer, nullable=False)
    email: Mapped[str] = mapped_column(String(255), nullable=False)
    generation: Mapped[int] = mapped_column(BigInteger, nullable=False)
    client_state: Mapped[str] = mapped_column(String(32), nullable=False)
    created_at: Mapped[int] = mapped_column(BigInteger, nullable=False)
    replaced_at: Mapped[int] = mapped_column(BigInteger, default=Null)
    nodeid: Mapped[int] = mapped_column(BigInteger, nullable=False)
    keys_changed_at: Mapped[int] = mapped_column(BigInteger, default=Null)

    lookup_idx = Index(email, service, created_at)
    replaced_at_idx = Index(service, replaced_at)
    node_idx = Index(nodeid)

    def __repr__(self) -> str:
        return f"Users(uid={self.uid!r}, service={self.service!r}, email={self.email!r}, generation={self.generation!r}, client_state={self.client_state!r}, created_at={self.created_at!r}, replaced_at={self.replaced_at!r}, nodeid={self.nodeid!r}, keys_changed_at={self.keys_changed_at!r})"

    def _asdict(self):
        return {
            "uid": self.uid,
            "service": self.service,
            "email": self.email,
            "generation": self.generation,
            "client_state": self.client_state,
            "created_at": self.created_at,
            "replaced_at": self.replaced_at,
            "nodeid": self.nodeid,
            "keys_changed_at": self.keys_changed_at,
        }
