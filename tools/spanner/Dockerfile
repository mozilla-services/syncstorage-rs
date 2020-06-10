FROM python:3.7.7-buster

COPY purge_ttl.py count_expired_rows.py count_users.py requirements.txt /app/

RUN pip install -r /app/requirements.txt

USER nobody

ENTRYPOINT ["/usr/local/bin/python"]
CMD ["/app/purge_ttl.py"]
