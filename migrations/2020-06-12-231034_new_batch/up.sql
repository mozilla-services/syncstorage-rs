DROP TABLE `batches`;

CREATE TABLE `batch_uploads` (
  `batch` bigint(20)   NOT NULL,
  `userid` bigint(20)  NOT NULL,
  `collection` int(11) NOT NULL,
  PRIMARY KEY (`batch`, `userid`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

CREATE TABLE `batch_upload_items` (
  `batch` bigint(20)                   NOT NULL,
  `userid` bigint(20)                  NOT NULL,
  `id` varchar(64)                     NOT NULL,
  `sortindex` int(11)    DEFAULT NULL,
  `payload` mediumtext,
  `payload_size` int(11) DEFAULT NULL,
  `ttl_offset` int(11)   DEFAULT NULL,
  PRIMARY KEY (`batch`, `userid`, `id`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;
