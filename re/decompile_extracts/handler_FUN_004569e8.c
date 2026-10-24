// ==== FUN_004569e8 @ 004569e8 ====

undefined8
FUN_004569e8(longlong param_1,undefined8 param_2,short param_3,undefined8 param_4,
            undefined4 *param_5)

{
  HWND hWndInsertAfter;
  
  if ((param_3 == 0) || (DAT_00550fe8 == (HWND)0x0)) {
    DAT_00550fe8 = GetFocus();
  }
  else {
    *(undefined4 *)(param_1 + 0x5168) = 1;
    *(undefined4 *)(param_1 + 0x5c80) = 1;
    *(undefined4 *)(param_1 + 0x6798) = 1;
    *(undefined4 *)(param_1 + 0x72b0) = 1;
    SetFocus(DAT_00550fe8);
  }
  *param_5 = 1;
  if (*(int *)(param_1 + 0x4e24) != 0) {
    hWndInsertAfter = (HWND)0xffffffffffffffff;
    if (param_3 == 0) {
      hWndInsertAfter = (HWND)0xfffffffffffffffe;
    }
    SetWindowPos(*(HWND *)(param_1 + 8),hWndInsertAfter,0,0,0,0,0x13);
  }
  return 0;
}


