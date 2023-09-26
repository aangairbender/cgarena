import React from 'react';
import ReactDOM from 'react-dom/client';
import {
  createBrowserRouter,
  RouterProvider,
} from 'react-router-dom';
import './index.css';
import reportWebVitals from './reportWebVitals';
import Root from './routes/root';
import Bots from './routes/bots';

import 'bootstrap/dist/css/bootstrap.min.css';
import AddBot from './routes/add_bot';

const router = createBrowserRouter([
  {
    path: "/",
    element: <Root />,
    children: [
      {
        path: "/bots",
        element: <Bots />,
      },
      {
        path: "/bots/:botId",
        element: <div>Not ready yet</div>,
      },
      {
        path: "/bots/add",
        element: <AddBot />,
      },
    ],
  },
]);

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(
  <React.StrictMode>
    <RouterProvider router={router}/>
  </React.StrictMode>
);

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
