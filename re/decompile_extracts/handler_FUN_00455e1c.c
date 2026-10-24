// ==== FUN_00455e1c @ 00455e1c ====

void FUN_00455e1c(undefined8 param_1)

{
  int iVar1;
  
  DAT_00551ac8 = FUN_0047dcfc(L"LUPE",L"cx",DAT_00551ac8,-1,-1);
  DAT_00551acc = FUN_0047dcfc(L"LUPE",L"cy",DAT_00551acc,-1,-1);
  iVar1 = FUN_0047dcfc(L"LUPE",L"zoom",(int)DAT_00551ad8,-1,-1);
  DAT_00551ad8 = (float)iVar1;
  DAT_00551a6c = FUN_0047dcfc(L"LUPE",L"zoom_mode",DAT_00551a6c,-1,-1);
  DAT_00551a8c = FUN_0047dcfc(L"LUPE",L"full_screen",DAT_00551a8c,-1,-1);
  DAT_00551a90 = FUN_0047dcfc(L"LUPE",L"window_border",DAT_00551a90,-1,-1);
  FUN_0050b568(0x551a20,DAT_00551ac8,DAT_00551acc,DAT_00551ad8,param_1);
  FUN_0050a4cc(0x551a20);
  return;
}


