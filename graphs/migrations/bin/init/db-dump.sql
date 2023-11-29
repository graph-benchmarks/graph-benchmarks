--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1
-- Dumped by pg_dump version 16.1 (Homebrew)

-- Started on 2023-11-29 17:09:29 CET

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
-- TOC entry 3350 (class 1262 OID 5)
-- Name: postgres; Type: DATABASE; Schema: -; Owner: user
--

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
-- TOC entry 215 (class 1259 OID 16384)
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
-- TOC entry 3344 (class 0 OID 16384)
-- Dependencies: 215
-- Data for Name: driver_logging; Type: TABLE DATA; Schema: public; Owner: user
--

INSERT INTO public.driver_logging VALUES (1, 'algo1', 'dataset1', 2, 10, 1024, 'runtime', 5000);
INSERT INTO public.driver_logging VALUES (2, 'algo1', 'dataset2', 2, 10, 1024, 'runtime', 4000);
INSERT INTO public.driver_logging VALUES (3, 'algo1', 'dataset3', 2, 10, 1024, 'runtime', 4500);
INSERT INTO public.driver_logging VALUES (4, 'algo1', 'dataset4', 2, 10, 1024, 'runtime', 2500);
INSERT INTO public.driver_logging VALUES (5, 'algo2', 'dataset1', 2, 10, 1024, 'runtime', 4800);
INSERT INTO public.driver_logging VALUES (6, 'algo2', 'dataset2', 2, 10, 1024, 'runtime', 4000);
INSERT INTO public.driver_logging VALUES (7, 'algo2', 'dataset3', 2, 10, 1024, 'runtime', 5200);
INSERT INTO public.driver_logging VALUES (8, 'algo2', 'dataset4', 2, 10, 1024, 'runtime', 3000);
INSERT INTO public.driver_logging VALUES (9, 'algo3', 'dataset1', 2, 10, 1024, 'runtime', 5400);
INSERT INTO public.driver_logging VALUES (10, 'algo3', 'dataset2', 2, 10, 1024, 'runtime', 4500);
INSERT INTO public.driver_logging VALUES (11, 'algo3', 'dataset3', 2, 10, 1024, 'runtime', 4800);
INSERT INTO public.driver_logging VALUES (12, 'algo3', 'dataset4', 2, 10, 1024, 'runtime', 3500);
INSERT INTO public.driver_logging VALUES (13, 'algo4', 'dataset1', 2, 10, 1024, 'runtime', 4500);
INSERT INTO public.driver_logging VALUES (14, 'algo4', 'dataset2', 2, 10, 1024, 'runtime', 3800);
INSERT INTO public.driver_logging VALUES (15, 'algo4', 'dataset3', 2, 10, 1024, 'runtime', 4200);
INSERT INTO public.driver_logging VALUES (16, 'algo4', 'dataset4', 2, 10, 1024, 'runtime', 2400);


--
-- TOC entry 3200 (class 2606 OID 16390)
-- Name: driver_logging structure_driver_logging_pk; Type: CONSTRAINT; Schema: public; Owner: user
--

ALTER TABLE ONLY public.driver_logging
    ADD CONSTRAINT structure_driver_logging_pk PRIMARY KEY (id);


-- Completed on 2023-11-29 17:09:29 CET

--
-- PostgreSQL database dump complete
--

