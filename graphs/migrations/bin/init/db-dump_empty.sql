--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1
-- Dumped by pg_dump version 16.1 (Homebrew)

-- Started on 2023-11-23 19:50:25 CET

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

DROP DATABASE postgres;
--
-- TOC entry 3350 (class 1262 OID 5)
-- Name: postgres; Type: DATABASE; Schema: -; Owner: user
--

CREATE DATABASE postgres WITH TEMPLATE = template0 ENCODING = 'UTF8' LOCALE_PROVIDER = libc LOCALE = 'en_US.utf8';


ALTER DATABASE postgres OWNER TO "user";

\connect postgres

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- TOC entry 3351 (class 0 OID 0)
-- Dependencies: 3350
-- Name: DATABASE postgres; Type: COMMENT; Schema: -; Owner: user
--

COMMENT ON DATABASE postgres IS 'default administrative connection database';


--
-- TOC entry 4 (class 2615 OID 2200)
-- Name: public; Type: SCHEMA; Schema: -; Owner: pg_database_owner
--

CREATE SCHEMA IF NOT EXISTS public;


ALTER SCHEMA public OWNER TO pg_database_owner;

--
-- TOC entry 3352 (class 0 OID 0)
-- Dependencies: 4
-- Name: SCHEMA public; Type: COMMENT; Schema: -; Owner: pg_database_owner
--

COMMENT ON SCHEMA public IS 'standard public schema';


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- TOC entry 215 (class 1259 OID 24576)
-- Name: driver_logging; Type: TABLE; Schema: public; Owner: user
--

CREATE TABLE public.driver_logging (
    id integer NOT NULL,
    algo character varying(256),
    dataset character varying(256),
    cpu integer,
    workers integer,
    mem_size integer,
    type character varying(8),
    "time" integer
);


ALTER TABLE public.driver_logging OWNER TO "user";

--
-- TOC entry 3344 (class 0 OID 24576)
-- Dependencies: 215
-- Data for Name: driver_logging; Type: TABLE DATA; Schema: public; Owner: user
--



--
-- TOC entry 3200 (class 2606 OID 24582)
-- Name: driver_logging structure_driver_logging_pk; Type: CONSTRAINT; Schema: public; Owner: user
--

ALTER TABLE ONLY public.driver_logging
    ADD CONSTRAINT structure_driver_logging_pk PRIMARY KEY (id);


-- Completed on 2023-11-23 19:50:25 CET

--
-- PostgreSQL database dump complete
--

