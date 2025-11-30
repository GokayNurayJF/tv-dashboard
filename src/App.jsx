import './App.css';
import {Settings} from './components/Settings.jsx';
import {useEffect, useRef, useState} from 'react';
import {invoke} from "@tauri-apps/api/core";
import {listen, TauriEvent} from "@tauri-apps/api/event";

const App = () => {
    const displayUrlsRef = useRef(['']);
    const [index, setIndex] = useState(-1);
    const [time, setTime] = useState(0);
    const endTimeRef = useRef(0);
    const intervalRef = useRef(0);
    const intervalIdRef = useRef(null);

    const resetCounter = () => {
        endTimeRef.current = Date.now() + intervalRef.current;
    };

    const increaseIndex = () => {
        setIndex((prevIndex) => {
            return (prevIndex + 1) % displayUrlsRef.current.length;
        });
        resetCounter();
    }

    const decreaseIndex = () => {
        setIndex((prevIndex) => {
            return (prevIndex - 1 + displayUrlsRef.current.length) % displayUrlsRef.current.length;
        });
        resetCounter();
    }

    useEffect(() => {
        setInterval(() => {
            setTime(Date.now());
        }, 0);

        const unlisten = listen('reset-timer', () => {
            resetCounter();
            invoke("set_page_change_timestamp", {timestamp: endTimeRef.current});
        });

        const unlisten2 = listen('keyup', (event) => {
            document.dispatchEvent(new KeyboardEvent('keyup', {key : event.payload}));
        });

        const unlisten3 = listen('change-index', (event) => {
            console.log('Change index event received:', event);
            const newIndex = event.payload;
            if (newIndex !== index) {
                setIndex(newIndex);
                resetCounter();
            }
        });

        listen(TauriEvent.WINDOW_DESTROYED, () => {
            setIndex(-1);
            endTimeRef.current = 0;
            clearInterval(intervalIdRef.current);
        });

        const onKeyUp = (e) => {
            const urls = displayUrlsRef.current;
            if (urls.length === 0 || endTimeRef.current === 0) return;
            if (e.key === 'ArrowRight') increaseIndex();
            if (e.key === 'ArrowLeft') decreaseIndex();
        };
        document.addEventListener('keyup', onKeyUp);

        return () => {
            unlisten.then(unlisten => unlisten());
            unlisten2.then(unlisten => unlisten());
            unlisten3.then(unlisten => unlisten());
            document.removeEventListener('keyup', onKeyUp);
            if (intervalIdRef.current) {
                clearInterval(intervalIdRef.current);
            }
        }
    }, []);

    const startLoop = (urls, timeBetween) => {
        sessionStorage.setItem('urls', JSON.stringify(urls));
        invoke("create_window", {urls});

        urls = urls.filter(url => url.trim() !== '');
        displayUrlsRef.current = urls;
        intervalRef.current = timeBetween;
        setIndex(0);
        endTimeRef.current = Date.now() + timeBetween;

        if (intervalIdRef.current) {
            clearInterval(intervalIdRef.current);
        }

        intervalIdRef.current = setInterval(() => {
            if (Date.now() >= endTimeRef.current + 1000) {
                //send change signal if change from screen did not work
                console.log("Stuck at index", index);
                endTimeRef.current = Date.now() + timeBetween;
                invoke("change_url", {index: 0, endTime: endTimeRef.current});
            }
        }, 100);
    };

    useEffect(() => {
        const urls = displayUrlsRef.current;
        if (urls[index]) {
            invoke("change_url", {index, endTime: endTimeRef.current});
        }
    }, [index]);

    return (
        <div className="content">
            <div className="counter-section">
                {endTimeRef.current > 0 && (
                    <>
                        <p className="counter-text">
                            {endTimeRef.current - time > 0 ? ((endTimeRef.current - time) / 1000).toFixed() : "0.0"}
                        </p>
                        <p className="counter-desc">Seconds to next page</p>
                    </>
                )}
            </div>
            <div className="settings">
                <Settings startLoop={startLoop} currentIndex={index}/>
            </div>
            <p className="description-text">
                Use <kbd>←</kbd> and <kbd>→</kbd> arrow keys to quickly navigate through your pages.
                <br/>
                Interact with pages using the mouse or keyboard to reset the timer and prevent the page from changing.
            </p>
        </div>
    );
};

export default App;